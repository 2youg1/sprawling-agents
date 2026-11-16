// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Real CPython-WASI execution readings; WAT correctness fixtures are
//! never performance substitutes (`Tools/citysim/spec/Bench.lean` §8-6-1).

use std::io::Write as _;
use std::path::{Path, PathBuf};

use std::sync::LazyLock;
use std::time::Duration;

use kernel::B3Hash;
use runtime::{
    Fuel, GuestMemory, Mount, PythonGuest, Sandbox as _, SandboxExit, SandboxJob, WasmtimeSandbox,
};

mod cold;
mod concurrent;
mod reporting;
pub(crate) use cold::COLD_COMMAND;
use concurrent::memory_line;
use reporting::{memory_observations, round_observations};

/// Pilot counts for a baseline capture; a final comparison chooses precision
/// from its pre-registered interval width instead of treating these as enough.
pub(super) const WARM_SAMPLES: usize = 20;
pub(super) const CONCURRENT_CALLERS: usize = 4;
pub(super) const CONCURRENT_BURSTS: usize = 5;
/// The process-table reader is deliberately not run more often than gauge's
/// shortest beat: a finer reading can disturb the latency it is measuring.
pub(super) const MEMORY_BEAT: Duration = sprawling::monitor::tree::MINIMUM_BEAT;

pub(super) const STATE_WRITTEN: &str = "sprawling-state-written";
pub(super) const STATE_CLEAN: &str = "sprawling-state-clean";
pub(super) const STATE_LEAKED: &str = "sprawling-state-leaked";

/// A deterministic resident-shaped script against the advertised stdlib. Its
/// implementation assertion and exact output validate the configured fixture;
/// S2 still owns supply-chain provenance.
pub(super) const MARKER: &str = "sprawling-python-bench-v1:";
pub(super) const EXPECTED_SUFFIX: &str = "7:2:alpha,beta\n";
pub(super) static EXPECTED: LazyLock<Vec<u8>> =
    LazyLock::new(|| format!("{MARKER}{EXPECTED_SUFFIX}").into_bytes());
pub(super) static SCRIPT: LazyLock<String> = LazyLock::new(|| {
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

pub(super) struct Measurement {
    lines: Vec<String>,
    observations: Vec<Observation>,
}

#[derive(Clone, Copy)]
pub(super) struct Sample {
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
pub(super) struct Observation {
    row: String,
    index: usize,
    round: usize,
    metric: &'static str,
    value: u64,
    unit: &'static str,
    identity: Identity,
}

/// The guest selected for measurement and the identity every line carries.
pub(super) struct Guest {
    path: PathBuf,
    digest: B3Hash,
    stdlib: Mount,
    /// Owns the snapshot directory for every call in this measurement.
    _snapshot: tempfile::TempDir,
}

/// Two digests always travel together: neither a guest nor a script reading
/// is comparable when the other input changed.
#[derive(Debug, Clone, Copy)]
pub(super) struct Identity {
    build: B3Hash,
    guest: B3Hash,
    script: B3Hash,
    arm: Arm,
}

pub(super) struct Prepared {
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
pub(crate) fn cold_child(
    expected_guest: &str,
    snapshot: &Path,
    stdlib: &Path,
) -> Result<(), String> {
    let selected = PythonGuest::with_stdlib(snapshot.to_path_buf(), stdlib.to_path_buf());
    let prepared = prepare(&selected)?;
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

pub(super) fn measure_from(wasm: Option<PathBuf>, arm: Arm) -> Result<Report, String> {
    let wasm = wasm
        .ok_or_else(|| prerequisite("the doctor found no usable CPython-WASI guest".to_owned()))?;
    measure_at_as(&wasm, arm)
}

pub(super) fn measure_at_as(wasm: &Path, arm: Arm) -> Result<Report, String> {
    measure_guest_as(PythonGuest::at(wasm.to_path_buf()), arm)
}

pub(super) fn measure_guest_as(selected: PythonGuest, arm: Arm) -> Result<Report, String> {
    let stdlib_before = stdlib_digest(&selected.stdlib().host)?;
    let mut prepared = prepare(&selected)?;
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
    if stdlib_digest(&selected.stdlib().host)? != stdlib_before {
        return Err(fixture(
            "the stdlib tree changed while the baseline was running",
        ));
    }
    Ok(report)
}

#[cfg(test)]
fn measure_at(wasm: &Path) -> Result<Report, String> {
    measure_at_as(wasm, Arm::Baseline)
}

impl Guest {
    /// Reads the configured source once, then executes only a bench-owned
    /// snapshot whose digest is the identity of those same bytes.
    fn at(selected: &PythonGuest) -> Result<Guest, String> {
        let source = selected.wasm();
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
            stdlib: selected.stdlib().clone(),
            _snapshot: snapshot,
        })
    }

    fn digest_at(path: &Path) -> Result<B3Hash, String> {
        std::fs::read(path)
            .map(|bytes| B3Hash::digest(&bytes))
            .map_err(|why| prerequisite(format!("reread the CPython guest snapshot: {why}")))
    }
}

/// Canonical regular-file entries from docs/third-party.md, including raw path bytes.
pub(super) fn stdlib_digest(root: &Path) -> Result<B3Hash, String> {
    fn visit(
        root: &Path,
        relative: Vec<u8>,
        entries: &mut Vec<(Vec<u8>, PathBuf)>,
    ) -> std::io::Result<()> {
        for entry in std::fs::read_dir(root)? {
            let entry = entry?;
            let mut name = relative.clone();
            if !name.is_empty() {
                name.push(b'/');
            }
            name.extend_from_slice(entry.file_name().as_encoded_bytes());
            let kind = entry.file_type()?;
            if kind.is_dir() {
                visit(&entry.path(), name, entries)?;
            } else if kind.is_file() {
                entries.push((name, entry.path()));
            }
        }
        Ok(())
    }
    let read = || -> std::io::Result<B3Hash> {
        let mut entries = Vec::new();
        visit(root, Vec::new(), &mut entries)?;
        entries.sort_by(|left, right| left.0.cmp(&right.0));
        let mut bytes = Vec::new();
        for (name, path) in entries {
            let contents = std::fs::read(path)?;
            let length = u64::try_from(contents.len()).map_err(std::io::Error::other)?;
            bytes.extend_from_slice(&name);
            bytes.push(0);
            bytes.extend_from_slice(&length.to_le_bytes());
            bytes.extend_from_slice(&contents);
        }
        Ok(B3Hash::digest(&bytes))
    };
    read().map_err(|why| {
        prerequisite(format!(
            "digest the selected stdlib tree {}: {why}",
            root.display()
        ))
    })
}

pub(super) fn current_build() -> Result<B3Hash, String> {
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
pub(super) fn prepare(selected: &PythonGuest) -> Result<Prepared, String> {
    let guest = Guest::at(selected)?;
    let mut sandbox = WasmtimeSandbox::new().map_err(|why| prerequisite(why.to_string()))?;
    run_checked(&mut sandbox, &guest).map_err(|why| {
        prerequisite(format!(
            "the selected guest is not a usable CPython-WASI fixture: {why}"
        ))
    })?;
    Ok(Prepared { guest, sandbox })
}

pub(super) fn raw_job(guest: &Guest, script: &str) -> SandboxJob {
    SandboxJob {
        wasm: guest.path.clone(),
        argv: vec!["python".to_owned(), "-c".to_owned(), script.to_owned()],
        env: Vec::new(),
        stdin: Vec::new(),
        mounts: vec![guest.stdlib.clone()],
        fuel: Fuel(kernel::consts_policy::SANDBOX_FUEL_DEFAULT),
        memory: GuestMemory::Wasm32,
    }
}

pub(super) fn run_checked(sandbox: &mut WasmtimeSandbox, guest: &Guest) -> Result<(), String> {
    let outcome = sandbox
        .run(&raw_job(guest, SCRIPT.as_str()))
        .map_err(|why| why.to_string())?;
    output_detail(&outcome)
}

pub(super) struct TimedOutcome {
    took: Duration,
    returned: std::time::Instant,
    outcome: runtime::SandboxOutcome,
}

pub(super) fn timed_outcome(
    sandbox: &mut WasmtimeSandbox,
    guest: &Guest,
) -> Result<TimedOutcome, String> {
    let started = super::stamp();
    let outcome = sandbox
        .run(&raw_job(guest, SCRIPT.as_str()))
        .map_err(|why| why.to_string())?;
    let returned = super::stamp();
    Ok(TimedOutcome {
        took: returned.duration_since(started),
        returned,
        outcome,
    })
}

pub(super) fn checked_output(outcome: &runtime::SandboxOutcome) -> Result<(), String> {
    output_detail(outcome).map_err(fixture)
}

/// Returns validation details before the caller assigns the phase-specific code.
pub(super) fn output_detail(outcome: &runtime::SandboxOutcome) -> Result<(), String> {
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

pub(super) use concurrent::{Memory, concurrent_calls, measure_memory, warm_calls};
pub(super) use reporting::{fixture, prerequisite, record};

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests;
