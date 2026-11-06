// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Real CPython-WASI execution readings; WAT correctness fixtures are
//! never performance substitutes (`tools/citysim/spec/Bench.lean` §8-6-1).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Barrier, mpsc};
use std::time::Duration;

use kernel::{AxCode, AxError, B3Hash};
use runtime::{Fuel, GuestMemory, Sandbox as _, SandboxExit, SandboxJob, WasmtimeSandbox};
use sprawling::monitor::spread::micros;
use sprawling::monitor::tree::{Tree, TreeReading};

use super::reading::{MachineClass, row_line, spread_of};

/// Enough cold processes to expose startup variation without letting the
/// Python family dominate an already broad `just bench` run.
const COLD_SAMPLES: usize = 5;
const WARM_SAMPLES: usize = 20;
const CONCURRENT_CALLERS: usize = 4;
const CONCURRENT_BURSTS: usize = 5;
const COLD_PATIENCE: Duration = Duration::from_secs(120);
const PROCESS_POLL: Duration = Duration::from_millis(10);
/// The process-table reader is deliberately not run more often than gauge's
/// shortest beat: a finer reading can disturb the latency it is measuring.
const MEMORY_BEAT: Duration = Duration::from_millis(250);

/// A deterministic resident-shaped script against the advertised stdlib.
/// Its implementation assertion and marker make the first call both the
/// measured work and the proof that the selected guest is CPython.
const SCRIPT: &str = r#"import collections
import difflib
import itertools
import re
import sys
assert sys.implementation.name == "cpython"
left = "alpha beta beta"
right = "alpha gamma beta"
counts = collections.Counter(re.findall(r"[a-z]+", left))
diff = list(difflib.unified_diff(left.split(), right.split()))
ordered = sorted(counts.items())
names = itertools.chain.from_iterable((name for name, _count in ordered))
print(f"sprawling-python-bench-v1:{len(diff)}:{counts['beta']}:{','.join(names)}")
"#;
const MARKER: &[u8] = b"sprawling-python-bench-v1:";

/// Python readings are returned only after the whole family succeeds.
#[derive(Debug)]
pub(crate) struct Report {
    pub(crate) lines: Vec<String>,
}

/// The guest selected for measurement and the identity every line carries.
struct Guest {
    path: PathBuf,
    digest: B3Hash,
}

/// Two digests always travel together: neither a guest nor a script reading
/// is comparable when the other input changed.
#[derive(Clone, Copy)]
struct Identity {
    guest: B3Hash,
    script: B3Hash,
}

struct Prepared {
    guest: Guest,
    sandbox: WasmtimeSandbox,
    expected: Vec<u8>,
}

/// Measures the external guest selected by the production doctor door.
///
/// # Errors
/// A missing or non-CPython guest is `E_TOOL_UNAVAILABLE`; no Python reading
/// is returned in either case.
pub(crate) fn measure() -> Result<Report, String> {
    measure_from(sprawling::doctor::usable_python_wasm())
}

/// Runs exactly one real CPython call in this fresh process. Success produces
/// no output; the parent observes only the process endpoint and exit status.
pub(crate) fn cold_child(expected_guest: &str) -> Result<(), String> {
    let wasm = sprawling::doctor::usable_python_wasm()
        .ok_or_else(|| prerequisite("the doctor found no usable CPython-WASI guest".to_owned()))?;
    let prepared = prepare(&wasm)?;
    if prepared.guest.digest.to_string() != expected_guest {
        return Err(fixture(
            "the cold child received different guest bytes from its parent",
        ));
    }
    Ok(())
}

fn measure_from(wasm: Option<PathBuf>) -> Result<Report, String> {
    let wasm = wasm
        .ok_or_else(|| prerequisite("the doctor found no usable CPython-WASI guest".to_owned()))?;
    measure_at(&wasm)
}

fn measure_at(wasm: &Path) -> Result<Report, String> {
    let mut prepared = prepare(wasm)?;
    let identity = Identity {
        guest: prepared.guest.digest,
        script: B3Hash::digest(SCRIPT.as_bytes()),
    };
    let mut lines = record(
        "python_cold_call",
        cold_calls(identity.guest)?,
        identity,
        " mode=no_prewarm",
    )?;
    lines.extend(record(
        "python_warm_call",
        warm_calls(&mut prepared)?,
        identity,
        "",
    )?);
    let concurrent = concurrent_calls(&prepared)?;
    lines.extend(record(
        "python_concurrent_call",
        concurrent.calls,
        identity,
        &format!(
            " concurrency={CONCURRENT_CALLERS} bursts={CONCURRENT_BURSTS} calls_per_s={} start_delay_p50_us={} start_delay_p99_us={}",
            concurrent.calls_per_s,
            micros(concurrent.start_delay_p50),
            micros(concurrent.start_delay_p99)
        ),
    )?);
    lines.push(memory_line(identity, measure_memory(&prepared)?)?);
    if Guest::at(wasm)?.digest != prepared.guest.digest {
        return Err(fixture(
            "the guest bytes changed while the baseline was running",
        ));
    }
    Ok(Report { lines })
}

impl Guest {
    fn at(path: &Path) -> Result<Guest, String> {
        let bytes = std::fs::read(path).map_err(|why| {
            prerequisite(format!(
                "read the CPython-WASI guest {}: {why}",
                path.display()
            ))
        })?;
        Ok(Guest {
            path: path.to_path_buf(),
            digest: B3Hash::digest(&bytes),
        })
    }
}

impl Identity {
    fn labels(self) -> String {
        format!("guest={} script={}", self.guest, self.script)
    }
}

/// Validates and compiles the guest with the first run of the real load. WAT
/// can pass the engine checks, but cannot produce this script's CPython marker.
fn prepare(wasm: &Path) -> Result<Prepared, String> {
    let guest = Guest::at(wasm)?;
    let mut sandbox = WasmtimeSandbox::new().map_err(|why| prerequisite(why.to_string()))?;
    let expected = run_checked(&mut sandbox, &guest.path, None).map_err(|why| {
        prerequisite(format!(
            "the selected guest is not a usable CPython-WASI fixture: {why}"
        ))
    })?;
    Ok(Prepared {
        guest,
        sandbox,
        expected,
    })
}

fn raw_job(wasm: &Path, script: &str) -> SandboxJob {
    SandboxJob {
        wasm: wasm.to_path_buf(),
        argv: vec!["python".to_owned(), "-c".to_owned(), script.to_owned()],
        env: Vec::new(),
        stdin: Vec::new(),
        mounts: Vec::new(),
        fuel: Fuel(kernel::consts_policy::SANDBOX_FUEL_DEFAULT),
        memory: GuestMemory::Wasm32,
    }
}

fn run_checked(
    sandbox: &mut WasmtimeSandbox,
    wasm: &Path,
    expected: Option<&[u8]>,
) -> Result<Vec<u8>, String> {
    let outcome = sandbox
        .run(&raw_job(wasm, SCRIPT))
        .map_err(|why| why.to_string())?;
    if outcome.exit != SandboxExit::Success {
        return Err(format!("the benchmark guest ended with {:?}", outcome.exit));
    }
    if !outcome.stderr.is_empty() {
        return Err("the benchmark guest wrote to stderr".to_owned());
    }
    if !outcome.stdout.starts_with(MARKER) {
        return Err("the benchmark guest did not produce the CPython marker".to_owned());
    }
    if let Some(expected) = expected
        && outcome.stdout != expected
    {
        return Err("repeated CPython calls produced different stdout".to_owned());
    }
    Ok(outcome.stdout)
}

fn cold_calls(guest: B3Hash) -> Result<Vec<Duration>, String> {
    (0..COLD_SAMPLES).map(|_| cold_call(guest)).collect()
}

#[expect(
    clippy::disallowed_methods,
    reason = "a developer bench spawns its own executable; it is not a city child"
)]
fn cold_call(guest: B3Hash) -> Result<Duration, String> {
    let started = super::stamp();
    let executable = std::env::current_exe().map_err(|why| why.to_string())?;
    let mut child = Command::new(executable)
        .arg("python-cold")
        .arg(guest.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|why| format!("start a cold CPython bench child: {why}"))?;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(why) => {
                return Err(stop_child(
                    &mut child,
                    format!("wait for a cold CPython bench child: {why}"),
                ));
            }
        }
        if started.elapsed() >= COLD_PATIENCE {
            return Err(stop_child(
                &mut child,
                format!(
                    "the cold CPython child did not finish within {} seconds",
                    COLD_PATIENCE.as_secs()
                ),
            ));
        }
        std::thread::sleep(PROCESS_POLL);
    };
    let output = child.wait_with_output().map_err(|why| why.to_string())?;
    if !status.success() {
        return Err(prerequisite(format!(
            "the cold CPython child exited with {status}: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    if !output.stdout.is_empty() {
        return Err(fixture(
            "the cold child wrote an unexpected stdout protocol",
        ));
    }
    Ok(started.elapsed())
}

fn stop_child(child: &mut std::process::Child, detail: String) -> String {
    let stopped = child
        .kill()
        .map_err(|why| format!("kill the cold CPython bench child: {why}"))
        .and_then(|()| {
            child
                .wait()
                .map_err(|why| format!("reap the cold CPython bench child: {why}"))
        });
    match stopped {
        Ok(_) => prerequisite(detail),
        Err(cleanup) => prerequisite(format!("{detail}; {cleanup}")),
    }
}

fn warm_calls(prepared: &mut Prepared) -> Result<Vec<Duration>, String> {
    check_state_isolation(prepared)?;
    (0..WARM_SAMPLES)
        .map(|_| {
            let started = super::stamp();
            run_checked(
                &mut prepared.sandbox,
                &prepared.guest.path,
                Some(&prepared.expected),
            )
            .map_err(fixture)?;
            Ok(started.elapsed())
        })
        .collect()
}

fn check_state_isolation(prepared: &mut Prepared) -> Result<(), String> {
    let write = raw_job(
        &prepared.guest.path,
        "bench_state = 41\nprint('sprawling-state-written')\n",
    );
    let written = prepared
        .sandbox
        .run(&write)
        .map_err(|why| why.to_string())?;
    if written.exit != SandboxExit::Success
        || written.stdout != b"sprawling-state-written\n"
        || !written.stderr.is_empty()
    {
        return Err(fixture(
            "the state-writing CPython call did not complete as written",
        ));
    }
    let read = raw_job(
        &prepared.guest.path,
        "print('sprawling-state-leaked' if 'bench_state' in globals() else 'sprawling-state-clean')\n",
    );
    let read = prepared.sandbox.run(&read).map_err(|why| why.to_string())?;
    if read.exit != SandboxExit::Success
        || read.stdout != b"sprawling-state-clean\n"
        || !read.stderr.is_empty()
    {
        return Err(fixture(
            "a second CPython interpreter observed its predecessor's global",
        ));
    }
    Ok(())
}

struct Call {
    latency: Duration,
    start_delay: Duration,
}

struct Concurrent {
    calls: Vec<Duration>,
    calls_per_s: u64,
    start_delay_p50: Duration,
    start_delay_p99: Duration,
}

struct Memory {
    before: TreeReading,
    peak_virtual_bytes: u64,
    peak_resident_bytes: u64,
    after: TreeReading,
    beats: u64,
}

fn concurrent_calls(prepared: &Prepared) -> Result<Concurrent, String> {
    let mut calls = Vec::new();
    let mut starts = Vec::new();
    let mut wall = Duration::ZERO;
    for _ in 0..CONCURRENT_BURSTS {
        let burst = concurrent_burst(prepared)?;
        wall = wall.saturating_add(burst.1);
        calls.extend(burst.0.iter().map(|call| call.latency));
        starts.extend(burst.0.into_iter().map(|call| call.start_delay));
    }
    let start_spread = spread_of(starts)?;
    let call_count = u64::try_from(calls.len()).unwrap_or(u64::MAX);
    let calls_per_s = call_count
        .saturating_mul(1_000_000)
        .checked_div(micros(wall).max(1))
        .unwrap_or(0);
    Ok(Concurrent {
        calls,
        calls_per_s,
        start_delay_p50: start_spread.p(sprawling::monitor::spread::Share::P50),
        start_delay_p99: start_spread.p(sprawling::monitor::spread::Share::P99),
    })
}

/// Memory is read in a separate burst so the process-table walk never
/// disturbs the latency and throughput samples.
fn measure_memory(prepared: &Prepared) -> Result<Memory, String> {
    let mut tree = Tree::open(std::process::id());
    let before = tree
        .read(Duration::ZERO)
        .ok_or_else(|| fixture("the memory reader could not find this bench process"))?;
    let (stop, stopped) = mpsc::channel();
    let monitor = std::thread::spawn(move || monitor_memory(tree, stopped));
    let measured: Result<(), String> = (|| {
        for _ in 0..CONCURRENT_BURSTS {
            concurrent_burst(prepared)?;
        }
        Ok(())
    })();
    let sent = stop.send(());
    let monitored = monitor.join();
    sent.map_err(|why| why.to_string())?;
    let (mut tree, beats) = monitored.map_err(|_| fixture("the memory monitor panicked"))??;
    let after = tree
        .read(Duration::ZERO)
        .ok_or_else(|| fixture("the memory reader lost this bench process after the burst"))?;
    let seen = tree
        .seen()
        .ok_or_else(|| fixture("the memory reader kept no readings"))?;
    measured?;
    Ok(Memory {
        before,
        peak_virtual_bytes: seen.peak_private_bytes,
        peak_resident_bytes: seen.peak_working_set_bytes,
        after,
        beats: beats.saturating_add(2),
    })
}

fn concurrent_burst(prepared: &Prepared) -> Result<(Vec<Call>, Duration), String> {
    let barrier = Barrier::new(CONCURRENT_CALLERS.saturating_add(1));
    let spawned_at = super::stamp();
    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(CONCURRENT_CALLERS);
        for _ in 0..CONCURRENT_CALLERS {
            let barrier = &barrier;
            let mut sandbox = prepared.sandbox.clone();
            let expected = &prepared.expected;
            let guest = &prepared.guest.path;
            handles.push(scope.spawn(move || {
                barrier.wait();
                let start_delay = spawned_at.elapsed();
                let started = super::stamp();
                run_checked(&mut sandbox, guest, Some(expected)).map_err(fixture)?;
                Ok::<Call, String>(Call {
                    latency: started.elapsed(),
                    start_delay,
                })
            }));
        }
        let started = super::stamp();
        barrier.wait();
        let joined: Vec<_> = handles.into_iter().map(|handle| handle.join()).collect();
        let mut calls = Vec::with_capacity(CONCURRENT_CALLERS);
        for result in joined {
            calls.push(result.map_err(|_| fixture("a concurrent CPython caller panicked"))??);
        }
        Ok((calls, started.elapsed()))
    })
}

fn monitor_memory(mut tree: Tree, stopped: mpsc::Receiver<()>) -> Result<(Tree, u64), String> {
    let mut beats = 0_u64;
    let mut previous = super::stamp();
    loop {
        match stopped.recv_timeout(MEMORY_BEAT) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => return Ok((tree, beats)),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let now = super::stamp();
                if tree.read(now.duration_since(previous)).is_none() {
                    return Err(fixture("the memory monitor lost this bench process"));
                }
                previous = now;
                beats = beats.saturating_add(1);
            }
        }
    }
}

fn memory_line(identity: Identity, memory: Memory) -> Result<String, String> {
    if memory.before.private_bytes == 0
        || memory.before.working_set_bytes == 0
        || memory.after.private_bytes == 0
        || memory.after.working_set_bytes == 0
    {
        return Err(fixture(
            "a platform memory counter answered zero for a live process",
        ));
    }
    Ok(format!(
        "python_memory machine_class={} os={} arch={} {} beats={} before_virtual_bytes={} peak_virtual_bytes={} after_virtual_bytes={} before_resident_bytes={} peak_resident_bytes={} after_resident_bytes={}",
        MachineClass::General.as_str(),
        std::env::consts::OS,
        std::env::consts::ARCH,
        identity.labels(),
        memory.beats,
        memory.before.private_bytes,
        memory.peak_virtual_bytes,
        memory.after.private_bytes,
        memory.before.working_set_bytes,
        memory.peak_resident_bytes,
        memory.after.working_set_bytes
    ))
}

fn record(
    row: &str,
    samples: Vec<Duration>,
    identity: Identity,
    beside: &str,
) -> Result<Vec<String>, String> {
    let mut lines: Vec<String> = samples
        .iter()
        .enumerate()
        .map(|(index, sample)| {
            format!(
                "python_sample row={row} index={} took_us={} {}",
                index.saturating_add(1),
                micros(*sample),
                identity.labels()
            )
        })
        .collect();
    let conditions = format!(
        " machine_class={} os={} arch={} {}{beside}",
        MachineClass::General.as_str(),
        std::env::consts::OS,
        std::env::consts::ARCH,
        identity.labels()
    );
    lines.push(row_line(row, &spread_of(samples)?, &conditions));
    Ok(lines)
}

fn prerequisite(detail: String) -> String {
    AxError::failure(
        AxCode::ToolUnavailable,
        "measure the real CPython-WASI baseline",
        detail,
    )
    .with_recovery(
        "configure a real CPython-WASI guest through the external guest door documented by \
         `sprawling doctor`, then rerun; a WAT correctness fixture is not a timing substitute",
    )
    .to_string()
}

fn fixture(detail: impl Into<String>) -> String {
    AxError::failure(
        AxCode::InvalidArgs,
        "measure the registered CPython-WASI scenario",
        detail.into(),
    )
    .with_recovery("check the selected guest and the fixed script; no partial report is emitted")
    .to_string()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_guest_is_a_prerequisite_error() {
        let refused = measure_from(None).expect_err("a missing guest produced a report");
        assert!(refused.contains("E_TOOL_UNAVAILABLE"), "{refused}");
    }

    #[test]
    fn an_invalid_guest_produces_no_reading() {
        let dir = tempfile::tempdir().unwrap();
        let wasm = dir.path().join("invalid.wasm");
        std::fs::write(&wasm, b"not WebAssembly").unwrap();
        let refused = measure_at(&wasm).expect_err("an invalid guest produced a report");
        assert!(refused.contains("E_TOOL_UNAVAILABLE"), "{refused}");
    }

    #[test]
    fn a_correctness_module_is_not_cpython_evidence() {
        // `(module (func (export "_start")))`: valid enough to run, but
        // it has no CPython implementation or benchmark output.
        const EMPTY_START: &[u8] = &[
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x04, 0x01, 0x60, 0x00, 0x00,
            0x03, 0x02, 0x01, 0x00, 0x07, 0x0a, 0x01, 0x06, b'_', b's', b't', b'a', b'r', b't',
            0x00, 0x00, 0x0a, 0x04, 0x01, 0x02, 0x00, 0x0b,
        ];
        let dir = tempfile::tempdir().unwrap();
        let wasm = dir.path().join("correctness.wasm");
        std::fs::write(&wasm, EMPTY_START).unwrap();
        let refused = measure_at(&wasm).expect_err("a WAT fixture produced a CPython report");
        assert!(refused.contains("CPython"), "{refused}");
    }
}
