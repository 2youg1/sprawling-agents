// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The four actions this binary prices, driven over the product's public faces
//! (`tools/citysim/spec/BenchStartup.lean` §8-5).
//!
//! Shape: adapter. Each driver is the timing boundary of its action and
//! nothing else: it stamps the boundary's ends and its sub-steps, counts
//! what landed on disk, and hands the readings to `samples`. The policy
//! that says where a boundary sits is in the SPEC, not here.
//!
//! Two of the four actions cross a process boundary on purpose (the
//! subject of ② is exactly that boundary), and both enter through a
//! plain `std::process::Command` of the shipped binary. What is measured
//! is the product's own path: no bench-only branch exists behind any of
//! these stamps.
//!
//! What a driver does not own sits in a sibling file: `archive` owns the
//! release archive's format, read and written, and `footprint` owns what
//! one sample left under a directory. Both are adapters over somebody
//! else's bytes; this file is the routing between them and the stamps.

#[path = "actions/archive.rs"]
pub(super) mod archive;
#[path = "actions/first_byte.rs"]
pub(super) mod first_byte;
#[path = "actions/footprint.rs"]
mod footprint;
#[path = "actions/history.rs"]
pub(super) mod history;

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use kernel::{Address, AxCode, AxError, IdemKey, RunId, Seq};

use super::samples::{Samples, Share};
use super::stamp;
use archive::{digest_mismatch, digest_of, unpack};
use footprint::{clear, files_under, ledger_lines};

/// How many times each action runs (`tools/citysim/spec/BenchStartup.lean` §8-5: the
/// floor is one hundred; two hundred is what puts p99 two samples below
/// the top rather than on it).
///
/// One number: the loops run to it, the vectors that hold their readings
/// reserve for it, and the report prints it.
pub(crate) const SAMPLES: usize = 200;

/// What one sample does to processes and to the disk.
pub struct PerSample {
    /// Processes created by one sample.
    pub processes: u64,
    /// How many files one sample wrote under the directory it was given.
    /// An action with no disk footprint of its own records zero rather
    /// than counting the tree it was pointed at.
    pub files: u64,
    /// Ledger lines written: one per durability barrier, because every
    /// append is its own barrier (the `durability_barrier` register row).
    pub barriers: u64,
}

/// One action's readings: the boundary, its sub-steps, and the counts.
pub struct Action {
    pub total: Samples,
    pub steps: Vec<(&'static str, Samples)>,
    pub per_sample: PerSample,
}

/// The sub-step that dominated: the largest middle reading among them.
///
/// `None` for an action whose boundary has no sub-step split (no product
/// instrumentation is added for this experiment); its report falls back
/// to counts times the barrier floor.
pub fn dominant(steps: &[(&'static str, Samples)]) -> Option<&'static str> {
    steps
        .iter()
        .max_by(|(_, left), (_, right)| left.p(Share::P50).cmp(&right.p(Share::P50)))
        .map(|(label, _)| *label)
}

/// ① install: archive in place to executable usable.
///
/// Boundary (citysim D3): the digest check `install.sh` and
/// `install.ps1` perform, the unpack, and the launch that confirms the
/// unpacked binary answers. The archive is prepared by the caller before
/// any of this runs: the reading starts at "the archive is in place".
pub fn install(scratch: &Path, archive_path: &Path) -> Result<Action, AxError> {
    let published = digest_of(archive_path)?;
    let mut totals = Vec::with_capacity(SAMPLES);
    let mut digests = Vec::with_capacity(SAMPLES);
    let mut unpacks = Vec::with_capacity(SAMPLES);
    let mut launches = Vec::with_capacity(SAMPLES);
    let mut per_sample = PerSample {
        processes: 0,
        files: 0,
        barriers: 0,
    };
    for sample in 0..SAMPLES {
        let into = scratch.join(format!("install-{sample}"));
        let boundary = stamp();
        let step = stamp();
        if digest_of(archive_path)? != published {
            return Err(digest_mismatch(archive_path));
        }
        digests.push(step.elapsed());
        let step = stamp();
        let binary = unpack(archive_path, &into)?;
        unpacks.push(step.elapsed());
        let step = stamp();
        spawn_version(&binary)?;
        launches.push(step.elapsed());
        totals.push(boundary.elapsed());
        if sample == 0 {
            per_sample = PerSample {
                processes: 1,
                files: files_under(&into)?,
                barriers: 0,
            };
        }
        clear(&into)?;
    }
    Ok(Action {
        total: collected(totals)?,
        steps: vec![
            ("digest", collected(digests)?),
            ("unpack", collected(unpacks)?),
            ("launch", collected(launches)?),
        ],
        per_sample,
    })
}

/// ② startup: `CreateProcess` sent to a light command's exit observed.
///
/// Sub-steps split where the operating system hands over: the spawn call
/// returning is process creation, and what follows is the program.
pub fn startup(binary: &Path) -> Result<Action, AxError> {
    let mut totals = Vec::with_capacity(SAMPLES);
    let mut creates = Vec::with_capacity(SAMPLES);
    let mut runs = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let boundary = stamp();
        let (create, run) = spawn_version(binary)?;
        totals.push(boundary.elapsed());
        creates.push(create);
        runs.push(run);
    }
    Ok(Action {
        total: collected(totals)?,
        steps: vec![("create", collected(creates)?), ("run", collected(runs)?)],
        per_sample: PerSample {
            processes: 1,
            files: 0,
            barriers: 0,
        },
    })
}

/// ③ raise a city: `init_city` called to the genesis record on disk.
///
/// Every sample raises a fresh city and is measured to the call
/// returning, which is after line zero and its barrier. The counts are
/// taken on the first city once its timing is over.
pub fn raise_city(scratch: &Path) -> Result<Action, AxError> {
    let mut totals = Vec::with_capacity(SAMPLES);
    let mut per_sample = PerSample {
        processes: 0,
        files: 0,
        barriers: 0,
    };
    for sample in 0..SAMPLES {
        let city = scratch.join(format!("city-{sample}"));
        let boundary = stamp();
        sprawling::assembly::init_city(&city)?;
        totals.push(boundary.elapsed());
        if sample == 0 {
            per_sample.files = files_under(&city)?;
            per_sample.barriers = ledger_lines(&kernel::layout::CityLayout::new(&city).ledger())?;
        }
    }
    Ok(Action {
        total: collected(totals)?,
        steps: Vec::new(),
        per_sample,
    })
}

/// ④ open a session: `Command::OpenSession` in to the call returning,
/// which is after `session_opened` has its barrier and the room can take
/// the next run.
///
/// The city, the building and the worker are raised before any stamp:
/// what is priced is the session, not the city it opens in. Each sample
/// takes the next sequence number rather than an index, so the run the
/// ledger sees is the contiguous one `Seq` documents.
pub fn open_session(city: &Path) -> Result<Action, AxError> {
    sprawling::assembly::init_city(city)?;
    let mut worker = accounting::worker::RunWorker::new(
        city,
        runtime::diagnostics::Diagnostics::off(),
        sprawling::assembly::hands(gateway::Custodian::in_memory()),
    )?;
    worker.handle(wire::Command::CreateBuilding {
        addr: Address::parse("lab")?,
        template: wire::TemplateName::parse("minimal")?,
        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"bench-create"),
    })?;
    let room = Address::parse("lab/room1")?;
    let ledger = kernel::layout::CityLayout::new(city).ledger();
    let files_before = files_under(city)?;
    let lines_before = ledger_lines(&ledger)?;
    let mut totals = Vec::with_capacity(SAMPLES);
    let mut per_sample = PerSample {
        processes: 0,
        files: 0,
        barriers: 0,
    };
    let mut seq = Seq::FIRST;
    for sample in 0..SAMPLES {
        seq = seq.next()?;
        let boundary = stamp();
        worker.handle(wire::Command::OpenSession {
            addr: room.clone(),
            carry: wire::Carry::Nothing,
            from: None,
            idem: IdemKey::derive(&RunId::CITY, seq, b"bench-open-session"),
        })?;
        totals.push(boundary.elapsed());
        if sample == 0 {
            // Both counts are differences, and both are taken after the
            // first sample's timing is over. A count that fell would mean
            // a session removed what an earlier one wrote, which nothing
            // in this action does; saturating keeps that out of the
            // report rather than wrapping it.
            per_sample.files = files_under(city)?.saturating_sub(files_before);
            per_sample.barriers = ledger_lines(&ledger)?.saturating_sub(lines_before);
        }
    }
    Ok(Action {
        total: collected(totals)?,
        steps: Vec::new(),
        per_sample,
    })
}

/// One action's readings, built from what the run recorded.
///
/// An action that recorded nothing is a measurement failure rather than
/// an empty set: `Samples` cannot be empty and this is where that is
/// said.
fn collected(times: Vec<Duration>) -> Result<Samples, AxError> {
    let mut times = times.into_iter();
    let Some(head) = times.next() else {
        return Err(AxError::failure(
            AxCode::EvidenceMissing,
            "take a reading",
            "an action recorded no samples",
        )
        .with_recovery("this is a defect in bench_startup: the loop did not run"));
    };
    Ok(Samples::of(head, times.collect()))
}

/// Runs the light command and waits for its exit.
///
/// Answers how long the spawn call took - process creation - and how
/// long the program then ran. The command is `version` because it is the
/// lightest one that still proves the process accepts commands.
fn spawn_version(binary: &Path) -> Result<(Duration, Duration), AxError> {
    let create = stamp();
    let mut child = Command::new(binary)
        .arg("version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| {
            AxError::failure(
                AxCode::ToolUnavailable,
                "run the measured binary",
                format!("{}: {err}", binary.display()),
            )
            .with_recovery("run just bench-startup, which builds the binary it measures first")
        })?;
    let create = create.elapsed();
    let run = stamp();
    let status = child.wait().map_err(|err| {
        AxError::failure(
            AxCode::ToolUnavailable,
            "wait for the measured binary",
            format!("{}: {err}", binary.display()),
        )
        .with_recovery("run just bench-startup again; the child did not report an exit")
    })?;
    let run = run.elapsed();
    if !status.success() {
        return Err(AxError::failure(
            AxCode::ToolUnavailable,
            "run the measured binary",
            format!("{}: `version` exited {status}", binary.display()),
        )
        .with_recovery("the measured binary does not answer its lightest command; rebuild it"));
    }
    Ok((create, run))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::{Samples, dominant};
    use std::time::Duration;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    /// The third piece of the three-piece delivery names the sub-step
    /// with the largest middle reading, and names nothing for an action
    /// whose boundary has no split.
    #[test]
    fn the_dominant_step_is_the_one_with_the_largest_middle() {
        let digest = ("digest", Samples::of(ms(1), vec![ms(1), ms(2)]));
        let launch = ("launch", Samples::of(ms(30), vec![ms(30), ms(40)]));
        assert_eq!(dominant(&[digest, launch]), Some("launch"));
        assert_eq!(dominant(&[]), None);
    }
}
