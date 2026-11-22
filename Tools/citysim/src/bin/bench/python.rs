// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::LazyLock;
use std::time::Duration;

use kernel::B3Hash;

mod cold;
mod concurrent;
mod reporting;
pub(crate) use cold::COLD_COMMAND;
use concurrent::memory_line;
use reporting::{memory_observations, round_observations};

pub(super) const WARM_SAMPLES: usize = 20;
pub(super) const CONCURRENT_CALLERS: usize = 4;
pub(super) const CONCURRENT_BURSTS: usize = 5;
pub(super) const MEMORY_BEAT: Duration = sprawling::monitor::tree::MINIMUM_BEAT;

pub(super) const STATE_WRITTEN: &str = "sprawling-state-written";
pub(super) const STATE_CLEAN: &str = "sprawling-state-clean";
pub(super) const STATE_LEAKED: &str = "sprawling-state-leaked";

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
    pub(crate) took: Duration,
    pub(crate) round: usize,
    pub(crate) metric: &'static str,
}

#[derive(Debug, Clone)]
pub(super) struct Observation {
    pub(crate) row: String,
    pub(crate) index: usize,
    pub(crate) round: usize,
    pub(crate) metric: &'static str,
    pub(crate) value: u64,
    pub(crate) unit: &'static str,
    pub(crate) identity: Identity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Arm {
    Baseline,
    Candidate,
}

impl Arm {
    pub(crate) fn parse(text: &str) -> Option<Arm> {
        match text {
            "baseline" => Some(Arm::Baseline),
            "candidate" => Some(Arm::Candidate),
            _ => None,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Arm::Baseline => "baseline",
            Arm::Candidate => "candidate",
        }
    }
}

#[derive(Clone)]
pub(super) struct Interpreter {
    pub(super) path: PathBuf,
    pub(super) digest: B3Hash,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Identity {
    pub(super) build: B3Hash,
    pub(super) guest: B3Hash,
    pub(super) script: B3Hash,
    pub(super) arm: Arm,
}

#[derive(Clone)]
pub(super) struct Prepared {
    pub(super) interpreter: Interpreter,
}

#[derive(Debug, Clone)]
pub(super) struct RunOutcome {
    pub(crate) exit_code: i32,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
}

pub(crate) fn measure() -> Result<Report, String> {
    measure_as(Arm::Baseline)
}

pub(crate) fn measure_as(arm: Arm) -> Result<Report, String> {
    measure_from(sprawling::doctor::usable_python(), arm)
}

pub(crate) fn cold_child(expected: &str, interpreter: &Path) -> Result<(), String> {
    let prepared = prepare_at(interpreter)?;
    if prepared.interpreter.digest.to_string() != expected {
        return Err(fixture(
            "the cold child received a different interpreter from its parent",
        ));
    }
    run_checked(&prepared.interpreter, SCRIPT.as_str())?;
    let mut out = std::io::stdout().lock();
    out.write_all(cold::COMPLETE_LINE.as_bytes())
        .and_then(|()| out.flush())
        .map_err(|why| format!("report the cold Python call's completion: {why}"))
}

pub(super) fn measure_from(python: Option<PathBuf>, arm: Arm) -> Result<Report, String> {
    let python = python.ok_or_else(|| {
        prerequisite("the doctor found no usable host Python interpreter".to_owned())
    })?;
    measure_at_as(&python, arm)
}

pub(super) fn measure_at_as(python: &Path, arm: Arm) -> Result<Report, String> {
    let mut prepared = prepare_at(python)?;
    let identity = Identity {
        build: current_build()?,
        guest: prepared.interpreter.digest,
        script: B3Hash::digest(SCRIPT.as_bytes()),
        arm,
    };
    let mut report = Report::new(arm);
    report.add(record(
        "python_cold_call",
        cold::calls(&prepared.interpreter)?,
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
    if Interpreter::digest_at(&prepared.interpreter.path)? != prepared.interpreter.digest {
        return Err(fixture(
            "the interpreter bytes changed while the baseline was running",
        ));
    }
    Ok(report)
}

#[cfg(test)]
fn measure_at(python: &Path) -> Result<Report, String> {
    measure_at_as(python, Arm::Baseline)
}

impl Interpreter {
    fn at(path: PathBuf) -> Result<Interpreter, String> {
        let digest = Self::digest_at(&path)?;
        Ok(Interpreter { path, digest })
    }

    fn digest_at(path: &Path) -> Result<B3Hash, String> {
        std::fs::read(path)
            .map(|bytes| B3Hash::digest(&bytes))
            .map_err(|why| {
                prerequisite(format!(
                    "read the host Python interpreter {}: {why}",
                    path.display()
                ))
            })
    }
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

pub(super) fn prepare_at(python: &Path) -> Result<Prepared, String> {
    let interpreter = Interpreter::at(python.to_path_buf())?;
    run_checked(&interpreter, SCRIPT.as_str()).map_err(|why| {
        prerequisite(format!(
            "the selected interpreter is not a usable CPython host fixture: {why}"
        ))
    })?;
    Ok(Prepared { interpreter })
}

pub(super) fn run_script(interpreter: &Interpreter, script: &str) -> Result<RunOutcome, String> {
    let output = Command::new(&interpreter.path)
        .arg("-c")
        .arg(script)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|why| {
            prerequisite(format!(
                "start host Python {}: {why}",
                interpreter.path.display()
            ))
        })?;
    Ok(RunOutcome {
        exit_code: output.status.code().unwrap_or(1),
        stdout: output.stdout,
        stderr: output.stderr,
    })
}

pub(super) fn run_checked(interpreter: &Interpreter, script: &str) -> Result<(), String> {
    let outcome = run_script(interpreter, script)?;
    output_detail(&outcome)
}

pub(super) struct TimedOutcome {
    pub(super) took: Duration,
    pub(super) returned: std::time::Instant,
    pub(super) outcome: RunOutcome,
}

pub(super) fn timed_outcome(interpreter: &Interpreter) -> Result<TimedOutcome, String> {
    let started = super::stamp();
    let outcome = run_script(interpreter, SCRIPT.as_str())?;
    let returned = super::stamp();
    Ok(TimedOutcome {
        took: returned.duration_since(started),
        returned,
        outcome,
    })
}

pub(super) fn checked_output(outcome: &RunOutcome) -> Result<(), String> {
    output_detail(outcome).map_err(fixture)
}

pub(super) fn output_detail(outcome: &RunOutcome) -> Result<(), String> {
    if outcome.exit_code != 0 {
        return Err(format!(
            "the benchmark interpreter exited with {}",
            outcome.exit_code
        ));
    }
    if !outcome.stderr.is_empty() {
        return Err("the benchmark interpreter wrote to stderr".to_owned());
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
