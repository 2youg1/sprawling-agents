// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Real CPython-WASI execution readings; WAT correctness fixtures are
//! never performance substitutes (`tools/citysim/spec/Bench.lean` §8-6-1).

use std::io::Write as _;
use std::path::{Path, PathBuf};

use std::sync::{Barrier, LazyLock, mpsc};
use std::time::Duration;

use kernel::{AxCode, AxError, B3Hash};
use runtime::{Fuel, GuestMemory, Sandbox as _, SandboxExit, SandboxJob, WasmtimeSandbox};
use sprawling::monitor::spread::micros;
use sprawling::monitor::tree::{Tree, TreeReading};

use super::reading::{MachineClass, row_line, spread_of};

mod cold;
pub(crate) use cold::COLD_COMMAND;

/// Pilot counts for a baseline capture; a final comparison chooses precision
/// from its pre-registered interval width instead of treating these as enough.
const WARM_SAMPLES: usize = 20;
const CONCURRENT_CALLERS: usize = 4;
const CONCURRENT_BURSTS: usize = 5;
/// The process-table reader is deliberately not run more often than gauge's
/// shortest beat: a finer reading can disturb the latency it is measuring.
const MEMORY_BEAT: Duration = sprawling::monitor::tree::MINIMUM_BEAT;

const STATE_WRITTEN: &str = "sprawling-state-written";
const STATE_CLEAN: &str = "sprawling-state-clean";
const STATE_LEAKED: &str = "sprawling-state-leaked";

/// A deterministic resident-shaped script against the advertised stdlib. Its
/// implementation assertion and exact output validate the configured fixture;
/// S2 still owns supply-chain provenance.
const MARKER: &str = "sprawling-python-bench-v1:";
const EXPECTED_SUFFIX: &str = "7:2:alpha,beta\n";
static EXPECTED: LazyLock<Vec<u8>> =
    LazyLock::new(|| format!("{MARKER}{EXPECTED_SUFFIX}").into_bytes());
static SCRIPT: LazyLock<String> = LazyLock::new(|| {
    format!(
        r#"import collections
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
names = itertools.chain.from_iterable([name] for name, _count in ordered)
print(f"{MARKER}{{len(diff)}}:{{counts['beta']}}:{{','.join(names)}}")
"#
    )
});

/// Python readings are returned only after the whole family succeeds.
#[derive(Debug)]
pub(crate) struct Report {
    pub(crate) lines: Vec<String>,
    observations: Vec<Observation>,
    arm: Arm,
}

struct Measurement {
    lines: Vec<String>,
    observations: Vec<Observation>,
}

#[derive(Clone, Copy)]
struct Sample {
    took: Duration,
    round: usize,
    metric: &'static str,
}

/// The two arms a later DiD analysis compares. The label is explicit in the
/// raw CSV rather than inferred from which file happened to be named first.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Arm {
    Baseline,
    Candidate,
}

impl Arm {
    /// `None` is an unknown CLI label; no arm is inferred from the filename.
    pub(crate) fn parse(raw: &str) -> Option<Arm> {
        match raw {
            "baseline" => Some(Arm::Baseline),
            "candidate" => Some(Arm::Candidate),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Arm::Baseline => "baseline",
            Arm::Candidate => "candidate",
        }
    }
}

/// One raw observation in the long-form CSV. Derived summaries are computed
/// from these rows rather than copied into a second table.
#[derive(Debug)]
struct Observation {
    row: String,
    index: usize,
    round: usize,
    metric: &'static str,
    value: u64,
    unit: &'static str,
    identity: Identity,
}

/// The guest selected for measurement and the identity every line carries.
struct Guest {
    path: PathBuf,
    digest: B3Hash,
    /// Owns the snapshot directory for every call in this measurement.
    _snapshot: tempfile::TempDir,
}

/// Two digests always travel together: neither a guest nor a script reading
/// is comparable when the other input changed.
#[derive(Debug, Clone, Copy)]
struct Identity {
    build: B3Hash,
    guest: B3Hash,
    script: B3Hash,
    arm: Arm,
}

struct Prepared {
    guest: Guest,
    sandbox: WasmtimeSandbox,
}

/// Measures the external guest selected by the production doctor door.
///
/// # Errors
/// A missing or non-CPython guest is `E_TOOL_UNAVAILABLE`; no Python reading
/// is returned in either case.
pub(crate) fn measure() -> Result<Report, String> {
    measure_as(Arm::Baseline)
}

/// Measures one explicitly labelled arm for a later matched comparison.
pub(crate) fn measure_as(arm: Arm) -> Result<Report, String> {
    measure_from(sprawling::doctor::usable_python_wasm(), arm)
}

/// Runs exactly one real CPython call in this fresh process, then writes and
/// flushes the completion line that ends the parent's timing.
pub(crate) fn cold_child(expected_guest: &str, snapshot: &Path) -> Result<(), String> {
    let prepared = prepare(snapshot)?;
    if prepared.guest.digest.to_string() != expected_guest {
        return Err(fixture(
            "the cold child received different guest bytes from its parent",
        ));
    }
    let mut out = std::io::stdout().lock();
    out.write_all(cold::COMPLETE_LINE.as_bytes())
        .and_then(|()| out.flush())
        .map_err(|why| format!("report the cold CPython call's completion: {why}"))
}

fn measure_from(wasm: Option<PathBuf>, arm: Arm) -> Result<Report, String> {
    let wasm = wasm
        .ok_or_else(|| prerequisite("the doctor found no usable CPython-WASI guest".to_owned()))?;
    measure_at_as(&wasm, arm)
}

#[cfg(test)]
fn measure_at(wasm: &Path) -> Result<Report, String> {
    measure_at_as(wasm, Arm::Baseline)
}

fn measure_at_as(wasm: &Path, arm: Arm) -> Result<Report, String> {
    let mut prepared = prepare(wasm)?;
    let identity = Identity {
        build: current_build()?,
        guest: prepared.guest.digest,
        script: B3Hash::digest(SCRIPT.as_bytes()),
        arm,
    };
    let mut report = Report::new(arm);
    report.add(record(
        "python_cold_call",
        cold::calls(&prepared.guest)?,
        identity,
        " mode=no_prewarm",
    )?);
    report.add(record(
        "python_warm_call",
        warm_calls(&mut prepared)?,
        identity,
        "",
    )?);
    let concurrent = concurrent_calls(&prepared)?;
    report.add(record(
        "python_concurrent_call",
        concurrent
            .calls
            .iter()
            .map(|call| Sample {
                took: call.latency,
                round: call.round,
                metric: "latency",
            })
            .collect(),
        identity,
        &format!(
            " concurrency={CONCURRENT_CALLERS} bursts={CONCURRENT_BURSTS} calls_per_s={}",
            concurrent.calls_per_s
        ),
    )?);
    report.add(record(
        "python_start_delay",
        concurrent
            .calls
            .iter()
            .map(|call| Sample {
                took: call.start_delay,
                round: call.round,
                metric: "start_delay",
            })
            .collect(),
        identity,
        &format!(" concurrency={CONCURRENT_CALLERS} bursts={CONCURRENT_BURSTS}"),
    )?);
    report
        .observations
        .extend(round_observations(identity, &concurrent.walls));
    let memory = measure_memory(&prepared)?;
    report.lines.push(memory_line(identity, &memory)?);
    report
        .observations
        .extend(memory_observations(identity, &memory));
    if Guest::digest_at(&prepared.guest.path)? != prepared.guest.digest {
        return Err(fixture(
            "the guest bytes changed while the baseline was running",
        ));
    }
    Ok(report)
}

impl Guest {
    /// Reads the configured source once, then executes only a bench-owned
    /// snapshot whose digest is the identity of those same bytes.
    fn at(source: &Path) -> Result<Guest, String> {
        let bytes = std::fs::read(source).map_err(|why| {
            prerequisite(format!(
                "read the CPython-WASI guest {}: {why}",
                source.display()
            ))
        })?;
        let snapshot = tempfile::tempdir().map_err(|why| {
            prerequisite(format!("create a private CPython guest snapshot: {why}"))
        })?;
        let path = snapshot.path().join("python.wasm");
        std::fs::write(&path, &bytes).map_err(|why| {
            prerequisite(format!("write the private CPython guest snapshot: {why}"))
        })?;
        Ok(Guest {
            path,
            digest: B3Hash::digest(&bytes),
            _snapshot: snapshot,
        })
    }

    fn digest_at(path: &Path) -> Result<B3Hash, String> {
        std::fs::read(path)
            .map(|bytes| B3Hash::digest(&bytes))
            .map_err(|why| prerequisite(format!("reread the CPython guest snapshot: {why}")))
    }
}

fn current_build() -> Result<B3Hash, String> {
    let executable = std::env::current_exe()
        .map_err(|why| prerequisite(format!("locate the measured bench executable: {why}")))?;
    std::fs::read(&executable)
        .map(|bytes| B3Hash::digest(&bytes))
        .map_err(|why| prerequisite(format!("read the measured bench executable: {why}")))
}

impl Identity {
    fn labels(self) -> String {
        format!("guest={} script={}", self.guest, self.script)
    }
}

/// Validates and compiles the snapshot with the first run of the real load.
/// WAT can pass the engine checks, but cannot produce the exact CPython output.
fn prepare(wasm: &Path) -> Result<Prepared, String> {
    let guest = Guest::at(wasm)?;
    let mut sandbox = WasmtimeSandbox::new().map_err(|why| prerequisite(why.to_string()))?;
    run_checked(&mut sandbox, &guest.path).map_err(|why| {
        prerequisite(format!(
            "the selected guest is not a usable CPython-WASI fixture: {why}"
        ))
    })?;
    Ok(Prepared { guest, sandbox })
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

fn run_checked(sandbox: &mut WasmtimeSandbox, wasm: &Path) -> Result<(), String> {
    let outcome = sandbox
        .run(&raw_job(wasm, SCRIPT.as_str()))
        .map_err(|why| why.to_string())?;
    checked_output(&outcome)
}

struct TimedOutcome {
    took: Duration,
    returned: std::time::Instant,
    outcome: runtime::SandboxOutcome,
}

fn timed_outcome(sandbox: &mut WasmtimeSandbox, wasm: &Path) -> Result<TimedOutcome, String> {
    let started = super::stamp();
    let outcome = sandbox
        .run(&raw_job(wasm, SCRIPT.as_str()))
        .map_err(|why| why.to_string())?;
    let returned = super::stamp();
    Ok(TimedOutcome {
        took: returned.duration_since(started),
        returned,
        outcome,
    })
}

fn checked_output(outcome: &runtime::SandboxOutcome) -> Result<(), String> {
    if outcome.exit != SandboxExit::Success {
        return Err(format!("the benchmark guest ended with {:?}", outcome.exit));
    }
    if !outcome.stderr.is_empty() {
        return Err("the benchmark guest wrote to stderr".to_owned());
    }
    if outcome.stdout != *EXPECTED {
        return Err("the fixed CPython script produced unexpected stdout".to_owned());
    }
    Ok(())
}

fn warm_calls(prepared: &mut Prepared) -> Result<Vec<Sample>, String> {
    check_state_isolation(prepared)?;
    (0..WARM_SAMPLES)
        .map(|index| {
            let measured = timed_outcome(&mut prepared.sandbox, &prepared.guest.path)?;
            checked_output(&measured.outcome)?;
            Ok(Sample {
                took: measured.took,
                round: index.saturating_add(1),
                metric: "latency",
            })
        })
        .collect()
}

fn check_state_isolation(prepared: &mut Prepared) -> Result<(), String> {
    let write_script = format!("bench_state = 41\nprint('{STATE_WRITTEN}')\n");
    let write = raw_job(&prepared.guest.path, &write_script);
    let written = prepared
        .sandbox
        .run(&write)
        .map_err(|why| why.to_string())?;
    if written.exit != SandboxExit::Success
        || written.stdout != output_line(STATE_WRITTEN)
        || !written.stderr.is_empty()
    {
        return Err(fixture(
            "the state-writing CPython call did not complete as written",
        ));
    }
    let read_script =
        format!("print('{STATE_LEAKED}' if 'bench_state' in globals() else '{STATE_CLEAN}')\n");
    let read = raw_job(&prepared.guest.path, &read_script);
    let read = prepared.sandbox.run(&read).map_err(|why| why.to_string())?;
    if read.exit != SandboxExit::Success
        || read.stdout != output_line(STATE_CLEAN)
        || !read.stderr.is_empty()
    {
        return Err(fixture(
            "a second CPython interpreter observed its predecessor's global",
        ));
    }
    Ok(())
}

fn output_line(marker: &str) -> Vec<u8> {
    format!("{marker}\n").into_bytes()
}

struct Call {
    latency: Duration,
    start_delay: Duration,
    round: usize,
    returned: std::time::Instant,
    outcome: runtime::SandboxOutcome,
}

struct Concurrent {
    calls: Vec<Call>,
    walls: Vec<Duration>,
    calls_per_s: u64,
}

struct Memory {
    before: TreeReading,
    peak_address_bytes: u64,
    peak_resident_bytes: u64,
    after: TreeReading,
    beats: u64,
}

/// `sysinfo` calls the same field virtual size on Unix but PrivateUsage on
/// Windows. The report names the measured counter rather than promising an
/// unavailable Windows address-space reservation reading.
enum VirtualCounter {
    Virtual,
    PrivateCommit,
}

impl VirtualCounter {
    fn current() -> VirtualCounter {
        if cfg!(windows) {
            VirtualCounter::PrivateCommit
        } else {
            VirtualCounter::Virtual
        }
    }

    fn names(self) -> [&'static str; 3] {
        match self {
            VirtualCounter::Virtual => ["before_virtual", "peak_virtual", "after_virtual"],
            VirtualCounter::PrivateCommit => [
                "before_private_commit",
                "peak_private_commit",
                "after_private_commit",
            ],
        }
    }
}

fn concurrent_calls(prepared: &Prepared) -> Result<Concurrent, String> {
    let mut calls = Vec::new();
    let mut walls = Vec::new();
    let mut wall_us = 0_u64;
    for round in 1..=CONCURRENT_BURSTS {
        let burst = concurrent_burst(prepared, round)?;
        wall_us = wall_us.saturating_add(micros(burst.1));
        walls.push(burst.1);
        calls.extend(burst.0);
    }
    let call_count = u64::try_from(calls.len()).unwrap_or(u64::MAX);
    let calls_per_s = call_count
        .saturating_mul(1_000_000)
        .checked_div(wall_us.max(1))
        .unwrap_or(0);
    Ok(Concurrent {
        calls,
        walls,
        calls_per_s,
    })
}

/// Memory is read in a separate burst so the process-table walk never
/// disturbs the latency and throughput samples.
fn measure_memory(prepared: &Prepared) -> Result<Memory, String> {
    let mut tree = Tree::open(std::process::id());
    let before = tree
        .read(Duration::ZERO)
        .ok_or_else(|| fixture("the memory reader could not find this bench process"))?;
    let before_at = super::stamp();
    let (stop, stopped) = mpsc::channel();
    let monitor = std::thread::spawn(move || monitor_memory(tree, stopped, before_at));
    let measured: Result<(), String> = (|| {
        for round in 1..=CONCURRENT_BURSTS {
            concurrent_burst(prepared, round)?;
        }
        Ok(())
    })();
    let sent = stop.send(()).map_err(|why| why.to_string());
    let monitored = monitor
        .join()
        .map_err(|_| fixture("the memory monitor panicked"))
        .and_then(|result| result);
    let cleanup = keep_both(sent, monitored).map(|((), monitored)| monitored);
    let ((), (mut tree, beats, last_read)) = keep_both(measured, cleanup)?;
    let now = super::stamp();
    let remaining = MEMORY_BEAT.saturating_sub(now.saturating_duration_since(last_read));
    if !remaining.is_zero() {
        std::thread::sleep(remaining);
    }
    let after_at = super::stamp();
    let after = tree
        .read(after_at.saturating_duration_since(last_read))
        .ok_or_else(|| fixture("the memory reader lost this bench process after the burst"))?;
    let seen = tree
        .seen()
        .ok_or_else(|| fixture("the memory reader kept no readings"))?;
    Ok(Memory {
        before,
        peak_address_bytes: seen.peak_private_bytes,
        peak_resident_bytes: seen.peak_working_set_bytes,
        after,
        beats: beats.saturating_add(2),
    })
}

fn concurrent_burst(prepared: &Prepared, round: usize) -> Result<(Vec<Call>, Duration), String> {
    let barrier = Barrier::new(CONCURRENT_CALLERS.saturating_add(1));
    let spawned_at = super::stamp();
    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(CONCURRENT_CALLERS);
        for _ in 0..CONCURRENT_CALLERS {
            let barrier = &barrier;
            let mut sandbox = prepared.sandbox.clone();
            let guest = &prepared.guest.path;
            handles.push(scope.spawn(move || {
                barrier.wait();
                let start_delay = spawned_at.elapsed();
                let measured = timed_outcome(&mut sandbox, guest)?;
                Ok::<Call, String>(Call {
                    latency: measured.took,
                    start_delay,
                    round,
                    returned: measured.returned,
                    outcome: measured.outcome,
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
        let wall = calls
            .iter()
            .map(|call| call.returned)
            .max()
            .map(|returned| returned.saturating_duration_since(started))
            .ok_or_else(|| fixture("a concurrent burst contained no calls"))?;
        for call in &calls {
            checked_output(&call.outcome)?;
        }
        Ok((calls, wall))
    })
}

fn monitor_memory(
    mut tree: Tree,
    stopped: mpsc::Receiver<()>,
    mut previous: std::time::Instant,
) -> Result<(Tree, u64, std::time::Instant), String> {
    let mut beats = 0_u64;
    loop {
        match stopped.recv_timeout(MEMORY_BEAT) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Ok((tree, beats, previous));
            }
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

/// Neither a measurement failure nor a cleanup failure erases the other.
fn keep_both<T, U>(work: Result<T, String>, cleanup: Result<U, String>) -> Result<(T, U), String> {
    match (work, cleanup) {
        (Ok(work), Ok(cleanup)) => Ok((work, cleanup)),
        (Err(work), Ok(_)) => Err(work),
        (Ok(_), Err(cleanup)) => Err(cleanup),
        (Err(work), Err(cleanup)) => Err(format!("{work}; {cleanup}")),
    }
}

fn memory_line(identity: Identity, memory: &Memory) -> Result<String, String> {
    if memory.before.private_bytes == 0
        || memory.before.working_set_bytes == 0
        || memory.after.private_bytes == 0
        || memory.after.working_set_bytes == 0
    {
        return Err(fixture(
            "a platform memory counter answered zero for a live process",
        ));
    }
    let [before_name, peak_name, after_name] = VirtualCounter::current().names();
    Ok(format!(
        "python_memory machine_class={} os={} arch={} {} beats={} {before_name}_bytes={} {peak_name}_bytes={} {after_name}_bytes={} before_resident_bytes={} peak_resident_bytes={} after_resident_bytes={}",
        MachineClass::General.as_str(),
        std::env::consts::OS,
        std::env::consts::ARCH,
        identity.labels(),
        memory.beats,
        memory.before.private_bytes,
        memory.peak_address_bytes,
        memory.after.private_bytes,
        memory.before.working_set_bytes,
        memory.peak_resident_bytes,
        memory.after.working_set_bytes
    ))
}

fn record(
    row: &str,
    samples: Vec<Sample>,
    identity: Identity,
    beside: &str,
) -> Result<Measurement, String> {
    let mut lines: Vec<String> = samples
        .iter()
        .enumerate()
        .map(|(index, sample)| {
            format!(
                "python_sample row={row} index={} took_us={} {}",
                index.saturating_add(1),
                micros(sample.took),
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
    let observations = samples
        .iter()
        .enumerate()
        .map(|(index, sample)| Observation {
            row: row.to_owned(),
            index: index.saturating_add(1),
            round: sample.round,
            metric: sample.metric,
            value: micros(sample.took),
            unit: "us",
            identity,
        })
        .collect();
    let durations = samples.iter().map(|sample| sample.took).collect();
    lines.push(row_line(row, &spread_of(durations)?, &conditions));
    Ok(Measurement {
        lines,
        observations,
    })
}

impl Report {
    fn add(&mut self, measurement: Measurement) {
        self.lines.extend(measurement.lines);
        self.observations.extend(measurement.observations);
    }

    fn new(arm: Arm) -> Report {
        Report {
            lines: Vec::new(),
            observations: Vec::new(),
            arm,
        }
    }

    /// Writes raw observations, not copied summaries, as one CSV artifact.
    ///
    /// # Errors
    /// Refuses an existing result and reports creation, write or publication failure
    /// without presenting the partial file as a completed measurement.
    pub(crate) fn write_csv(&self) -> Result<PathBuf, String> {
        let root = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("target"));
        let path = root.join(format!(
            "python-bench-{}-{}.csv",
            self.arm.as_str(),
            std::process::id()
        ));
        self.write_csv_at(&path)?;
        Ok(path)
    }

    fn write_csv_at(&self, path: &Path) -> Result<(), String> {
        let parent = path
            .parent()
            .ok_or_else(|| fixture("the CSV output path has no parent directory"))?;
        std::fs::create_dir_all(parent).map_err(|why| csv_storage(path, why))?;
        let part = path.with_extension("csv.part");
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&part)
            .map_err(|why| csv_storage(&part, why))?;
        let written = file
            .write_all(CSV_HEADER.as_bytes())
            .and_then(|()| {
                for observation in &self.observations {
                    observation.write_csv(&mut file)?;
                }
                Ok(())
            })
            .and_then(|()| file.sync_all());
        drop(file);
        // A same-directory hard link publishes atomically and fails when the
        // target already exists, unlike a replacement-style rename.
        let published = written.and_then(|()| std::fs::hard_link(&part, path));
        let cleanup = std::fs::remove_file(&part);
        keep_both(
            published.map_err(|why| csv_storage(path, why)),
            cleanup.map_err(|why| csv_storage(&part, why)),
        )?;
        Ok(())
    }
}

const CSV_SCHEMA_VERSION: u8 = 1;
const CSV_HEADER: &str = "schema_version,build_blake3,workload,observation,round,cohort,arm,metric,value,unit,guest_blake3,script_blake3,machine_class,os,arch\n";

impl Observation {
    fn write_csv(&self, out: &mut impl std::io::Write) -> std::io::Result<()> {
        writeln!(
            out,
            "{CSV_SCHEMA_VERSION},{},{},{},{},1,{},{},{},{},{},{},{},{},{}",
            self.identity.build,
            self.row,
            self.index,
            self.round,
            self.identity.arm.as_str(),
            self.metric,
            self.value,
            self.unit,
            self.identity.guest,
            self.identity.script,
            MachineClass::General.as_str(),
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    }
}

fn round_observations(identity: Identity, walls: &[Duration]) -> Vec<Observation> {
    walls
        .iter()
        .enumerate()
        .map(|(index, wall)| Observation {
            row: "python_concurrent_round".to_owned(),
            index: index.saturating_add(1),
            round: index.saturating_add(1),
            metric: "wall",
            value: micros(*wall),
            unit: "us",
            identity,
        })
        .collect()
}

fn memory_observations(identity: Identity, memory: &Memory) -> Vec<Observation> {
    let [before_name, peak_name, after_name] = VirtualCounter::current().names();
    [
        (before_name, memory.before.private_bytes),
        (peak_name, memory.peak_address_bytes),
        (after_name, memory.after.private_bytes),
        ("before_resident", memory.before.working_set_bytes),
        ("peak_resident", memory.peak_resident_bytes),
        ("after_resident", memory.after.working_set_bytes),
    ]
    .into_iter()
    .map(|(metric, value)| Observation {
        row: "python_memory".to_owned(),
        index: 1,
        round: 1,
        metric,
        value,
        unit: "bytes",
        identity,
    })
    .chain(std::iter::once(Observation {
        row: "python_memory".to_owned(),
        index: 1,
        round: 1,
        metric: "beats",
        value: memory.beats,
        unit: "count",
        identity,
    }))
    .collect()
}

fn csv_storage(path: &Path, detail: impl std::fmt::Display) -> String {
    AxError::failure(
        AxCode::StorageFatal,
        "write raw CPython-WASI measurements",
        format!("{}: {detail}", path.display()),
    )
    .with_recovery("choose a writable target directory and rerun; the partial CSV is not a result")
    .to_string()
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
        let refused =
            measure_from(None, Arm::Baseline).expect_err("a missing guest produced a report");
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

    #[test]
    fn the_csv_report_keeps_raw_observations_in_long_form() {
        let guest = B3Hash::digest(b"guest");
        let script = B3Hash::digest(b"script");
        let build = B3Hash::digest(b"build");
        let identity = Identity {
            build,
            guest,
            script,
            arm: Arm::Candidate,
        };
        let report = Report {
            lines: Vec::new(),
            observations: vec![Observation {
                row: "python_warm_call".to_owned(),
                index: 1,
                round: 2,
                metric: "latency",
                value: 17,
                unit: "us",
                identity,
            }],
            arm: Arm::Candidate,
        };
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("observations.csv");
        report.write_csv_at(&path).unwrap();
        assert_eq!(
            std::fs::read_to_string(path).unwrap(),
            format!(
                "{CSV_HEADER}1,{build},python_warm_call,1,2,1,candidate,latency,17,us,{guest},{script},general,{},{}\n",
                std::env::consts::OS,
                std::env::consts::ARCH
            )
        );
    }

    #[test]
    fn csv_publication_preserves_an_existing_target_and_removes_the_partial_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("observations.csv");
        std::fs::write(&path, b"previous measurement").unwrap();
        let part = path.with_extension("csv.part");
        let refused = Report::new(Arm::Baseline)
            .write_csv_at(&path)
            .expect_err("an existing target was replaced");
        assert!(refused.contains("E_STORAGE_FATAL"), "{refused}");
        assert_eq!(std::fs::read(path).unwrap(), b"previous measurement");
        assert!(!part.exists(), "the partial publication survived");
    }
}
