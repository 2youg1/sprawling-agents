// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Forming a city in a directory, and saying what was already there.

use std::path::{Path, PathBuf};

use city::{History, has_history};
use kernel::event::record::AutonomyChanged;
use kernel::{Address, AxCode, AxError, EventDraft, EventKind, EventRef};
use kernel::{Ledger, Payload, RunId};
use storage::JsonlLedger;

use super::freezing::Assembled;
use super::{Hands, RunWorker, ScanReport};

/// The city segment of every prefix, and a file the person is meant to
/// edit: `init` writes it into the city, and every later run reads that
/// copy. The binary carries the default so a fresh city is complete
/// without a checkout; its bytes are the city's template.
pub(super) const CITY_MD: &str = city::CITY_TEMPLATE;

#[derive(Debug)]
pub struct InitReport {
    pub ledger_dir: PathBuf,
    pub genesis: EventRef,
    /// What was already in the directory when the city formed, so the
    /// person who pointed at a year of their own work is told what was
    /// laid down beside it and what was left alone.
    pub standing: city::Standing,
    /// The folders that became buildings. Empty unless the caller asked
    /// for it: what is already on disk becomes governed only because
    /// somebody said so.
    pub adopted: Vec<Address>,
}

/// Whether the folders already in a directory become buildings.
///
/// Exhaustive rather than a flag, because the two are different acts: one
/// forms a city beside existing work and leaves it alone, and the other
/// puts that work under rules. A boolean would make them look like one
/// act with a setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Adopt {
    Nothing,
    EveryFolder,
}

/// What a directory holds, read from the directory itself.
///
/// The decision is `city::survey`'s; this only does the reading. A
/// directory that cannot be listed reads as empty, and the city forms -
/// the alternative is refusing to start over a permission error that the
/// next write would report anyway, with a better sentence.
pub(super) fn standing_of(city_root: &Path, history: History) -> city::Standing {
    let mut entries: Vec<(String, bool)> = Vec::new();
    if let Ok(listing) = std::fs::read_dir(city_root) {
        for entry in listing.flatten() {
            let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
            entries.push((entry.file_name().to_string_lossy().into_owned(), is_dir));
        }
    }
    city::survey(&entries, history == History::Present)
}

/// Forms a city in a directory with `hands`, and says what was already
/// there: the genesis write. The city is born when `city_initialized`
/// becomes line zero; a second forming refuses — history starts once.
///
/// `Adopt::EveryFolder` is the case a person with a workspace wants:
/// each top-level folder becomes a building with its own rules, its
/// files untouched. Adoption happens after genesis, because a building
/// is recorded against a city and there is no city before line zero.
///
/// The two genesis lines are stamped with the time `hands.clock` reads,
/// and the worker that raises City Hall is built with the same hands, so
/// a scripted clock and vault hold from line zero (accounting-SPEC.md
/// 8-3, 12-19).
///
/// # Errors
/// Refuses a directory that already has history, and propagates whatever
/// the ledger, the store or the filesystem says.
pub fn form(city_root: &Path, adopt: Adopt, hands: Hands) -> Result<InitReport, AxError> {
    let history = has_history(city_root)?;
    let standing = standing_of(city_root, history);
    let dir = kernel::layout::CityLayout::new(city_root).ledger();
    if history == History::Present {
        return Err(AxError::failure(
            AxCode::ConfigInvalid,
            "initialize city",
            dir.display().to_string(),
        )
        .with_recovery("this city already has history; open it, or init a fresh directory"));
    }
    std::fs::create_dir_all(&dir).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "create ledger directory",
            err.to_string(),
        )
        .with_recovery(
            "free space on this disk, or init the city in a directory this user may \
             write, then run `sprawling init` again",
        )
    })?;
    let now = crate::Clock::now(&*hands.clock)?;
    let (mut ledger, report) =
        JsonlLedger::open(&dir, now).map_err(storage::StorageError::into_ax)?;
    let genesis = ledger.append(EventDraft {
        run: RunId::CITY,
        t: now,
        who: "city".to_owned(),
        // The city's own name, recorded where every other fact about
        // this city is recorded. Without it the name lived only in a
        // directory entry, and every interface said "no city" over a
        // city that had been running for a month.
        addr: kernel::layout::CityLayout::new(city_root).city_address(),
        kind: EventKind::CityInitialized,
        data: Payload::of(&kernel::event::record::CityInitialized {})?,
        ig: false,
    })?;
    // The city's own records stay out of the workspace's git before
    // anything else is laid beside the project's files.
    city::ignore_city_records(city_root)?;
    let city_md = city_root.join(city::CITY_FILE);
    if !city_md.exists() {
        std::fs::write(&city_md, CITY_MD).map_err(|source| {
            AxError::failure(
                AxCode::StorageFatal,
                "write the city prompt",
                format!("{}: {source}", city_md.display()),
            )
            .with_recovery("check the city directory is writable")
        })?;
    }
    // Who answers the approval inbox, written down rather than baked
    // into a default: the person can appoint somebody else, and a
    // change needs a line to change.
    let delegation = AutonomyChanged {
        scope: kernel::event::Scope::City,
        autonomy: kernel::Autonomy::Delegate(
            kernel::ResidentId::new(kernel::consts_policy::HALL_CLERK).ok_or_else(|| {
                AxError::failure(
                    AxCode::ConfigInvalid,
                    "appoint the clerk",
                    kernel::consts_policy::HALL_CLERK.to_owned(),
                )
                .with_recovery("a resident id is a non-empty address")
            })?,
        ),
    };
    ledger.append(EventDraft {
        run: RunId::CITY,
        t: now,
        who: "city".to_owned(),
        addr: None,
        kind: EventKind::AutonomyChanged,
        data: Payload::of(&delegation)?,
        ig: false,
    })?;
    // City Hall, and the two identity files its residents are read
    // from. Both happen after line zero, because a building is recorded
    // against a city and there is no city before then.
    // The writer that wrote line zero goes on writing: a second one
    // opened here would be refused the city's writer lock.
    let mut worker = RunWorker::over(
        city_root,
        runtime::diagnostics::Diagnostics::off(),
        hands,
        (ledger, report),
    )?;
    let plan = city::CityPlan::new(None)?;
    let (hall, template) = plan.hall();
    worker.create_building(hall.clone(), template.name())?;
    city::lay_out_hall_identities(city_root)?;
    let mut adopted = Vec::new();
    if let (Adopt::EveryFolder, city::Standing::Work { adoptable, .. }) = (adopt, &standing) {
        // Through the same door `sprawling adopt` uses, so a folder
        // taken in at genesis and one taken in a month later end up
        // governed by the same rules.
        for addr in adoptable {
            worker.adopt_building(addr.clone())?;
            adopted.push(addr.clone());
        }
    }
    Ok(InitReport {
        ledger_dir: dir,
        genesis,
        standing,
        adopted,
    })
}

/// The city segment as this city has it: the file the person may edit,
/// falling back to the built-in copy when a city predates it.
///
/// The fallback is for one condition only. A city written before the
/// file existed does not have it, and the built-in copy is the right
/// answer there. Every other failure - a directory in its place, a
/// permission this process does not have - would have this hand a run
/// the built-in norms while the person's own edited norms sat unread on
/// the disk, and the run would obey the wrong document without anyone
/// being told.
///
/// # Errors
/// `E_STORAGE_FATAL` naming the path, for every failure except a file
/// that is not there.
pub(super) fn city_segment(city_root: &Path) -> Result<Assembled, AxError> {
    let path = city_root.join(city::CITY_FILE);
    match std::fs::read(&path) {
        Ok(bytes) => Ok(Assembled::of_one(Address::parse(city::CITY_FILE)?, bytes)),
        // The built-in copy names no document: no file on this disk
        // holds it, and a source row pointing at one that is not there
        // would send a reader to open nothing.
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Ok(Assembled::of_nothing(CITY_MD.as_bytes().to_vec()))
        }
        Err(err) => Err(AxError::failure(
            AxCode::StorageFatal,
            "read the city's norms",
            format!("{}: {err}", path.display()),
        )
        .with_recovery(
            "every run in this city is governed by that file; make it readable, \
             or take it away to fall back on the copy this build carries",
        )),
    }
}

impl RunWorker {
    /// Lays out a building, then records that it exists.
    ///
    /// The file lands before the event because the event says the
    /// building is there: a history that claims a directory nobody made
    /// would be replayed as confidently as a true one.
    pub(super) fn create_building(&mut self, addr: Address, template: &str) -> Result<(), AxError> {
        let template = city::BuildingTemplate::parse(template)?;
        let building = city::create_building(&self.city_root, &addr, template)?;
        self.note(
            runtime::diagnostics::Level::Effect,
            "city::building",
            &format!(
                "{} laid out from the {} template",
                building.addr().as_str(),
                template.name()
            ),
        );
        let payload = city::building_created_payload(&building, template)?;
        self.record(EventKind::BuildingCreated, payload)
    }

    /// Adopts a directory that already sits under the city as a
    /// building; the record says it was found, not built.
    ///
    /// # Errors
    /// Propagates what `city::adopt_building` reports — a path that is
    /// not a directory under this city among them — and whatever the
    /// ledger says about the record.
    pub fn adopt_building(&mut self, addr: Address) -> Result<(), AxError> {
        let building = city::adopt_building(&self.city_root, &addr)?;
        self.note(
            runtime::diagnostics::Level::Effect,
            "city::building",
            &format!("{} adopted as a building", building.addr().as_str()),
        );
        // The base checkpoint is paid here rather than by the first dispatch,
        // which would otherwise hash every file of the folder before its
        // first tool call (`crates/storage/Spec.lean` §8-8). No run and no model exist
        // yet, and an invented model id would be worse than none.
        let of = storage::Provenance::new(
            RunId::CITY,
            addr.clone(),
            self.city_hash()?,
            storage::ModelChoice {
                id: String::new(),
                effort: None,
            },
        );
        let t = crate::Clock::now(&*self.clock)?;
        storage::Checkpoint::open(&self.city_root)
            .and_then(|checkpoint| {
                checkpoint.base_checkpoint(&[addr.as_str().to_owned()], t, &of, &mut |step| {
                    self.note(
                        runtime::diagnostics::Level::Effect,
                        "storage::checkpoint",
                        &format!("{}: base checkpoint {step:?}", addr.as_str()),
                    );
                })
            })
            .map_err(storage::StorageError::into_ax)?;
        let payload = city::building_adopted_payload(&building)?;
        self.record(EventKind::BuildingCreated, payload)
    }

    /// The startup scan: closes the account of every
    /// tool call whose outcome the last process death left unknown,
    /// freezes every run that death left open, and reports what is still
    /// waiting on a person. Read-only apart from the closing `tool_result`
    /// drafts, which state E_TOOL_OUTCOME_UNKNOWN rather than guessing an
    /// outcome, and the freezes after them (accounting-SPEC.md 8-18-1).
    ///
    /// # Errors
    /// Propagates whatever the chain says about itself: a history that
    /// does not verify is not a history to append closing drafts to.
    pub fn startup_scan(&mut self) -> Result<ScanReport, AxError> {
        // Verifying the chain and finding the open calls are one
        // streamed pass: what stays resident is one segment's bytes and
        // the calls not yet closed, never the whole ledger.
        let mut calls = runtime::replay::DanglingCalls::default();
        let mut runs = lost::OpenRuns::default();
        let index = runtime::replay::fold_ledger_dir(
            &kernel::layout::CityLayout::new(&self.city_root).ledger(),
            |record| {
                calls.observe(record);
                runs.observe(record);
                Ok(())
            },
        )?;
        let mut closed = 0usize;
        for call in calls.into_calls() {
            let draft = runtime::replay::outcome_unknown_draft(&call, self.clock.now()?)?;
            self.ledger.append(draft)?;
            closed = closed.saturating_add(1);
        }
        // After the calls: a dead run's unknown outcomes belong to it, so
        // they land before the line that freezes it.
        let freezes = runs.into_drafts(self.clock.now()?)?;
        let frozen = freezes.len();
        for freeze in freezes {
            self.ledger.append(freeze)?;
        }
        Ok(ScanReport {
            opening: self.opening,
            lines: index.len(),
            closed_calls: closed,
            frozen_runs: frozen,
            waiting_approvals: self.governance.pending.len(),
        })
    }
}

mod lost;

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
    reason = "test code"
)]
mod tests;

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod adoption_tests;
