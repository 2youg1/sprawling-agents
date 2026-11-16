// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

pub(crate) const SAMPLES: usize = 200;

pub struct PerSample {
    pub processes: u64,

    pub files: u64,

    pub barriers: u64,
}

pub struct Action {
    pub total: Samples,
    pub steps: Vec<(&'static str, Samples)>,
    pub per_sample: PerSample,
}

pub fn dominant(steps: &[(&'static str, Samples)]) -> Option<&'static str> {
    steps
        .iter()
        .max_by(|(_, left), (_, right)| left.p(Share::P50).cmp(&right.p(Share::P50)))
        .map(|(label, _)| *label)
}

pub fn install(scratch: &Path, archive_path: &Path) -> Result<Action, AxError> {
    let published = digest_of(archive_path)?;
    let mut totals = Vec::with_capacity(SAMPLES);
    let mut digests = Vec::with_capacity(SAMPLES);
    let mut unpacks = Vec::with_capacity(SAMPLES);
    let mut systems = Vec::with_capacity(SAMPLES);
    let mut products = Vec::with_capacity(SAMPLES);
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
        let (system, product) = spawn_version(&binary)?;
        systems.push(system);
        products.push(product);
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
            ("system", collected(systems)?),
            ("product", collected(products)?),
        ],
        per_sample,
    })
}

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
        rules: None,
        source: None,
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

#[expect(clippy::disallowed_methods, reason = "developer tool (child D4)")]
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

    #[test]
    fn the_dominant_step_is_the_one_with_the_largest_middle() {
        let digest = ("digest", Samples::of(ms(1), vec![ms(1), ms(2)]));
        let launch = ("launch", Samples::of(ms(30), vec![ms(30), ms(40)]));
        assert_eq!(dominant(&[digest, launch]), Some("launch"));
        assert_eq!(dominant(&[]), None);
    }
}
