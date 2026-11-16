// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use city::{History, has_history};
use kernel::event::record::{
    AutonomyChanged, CheckpointCommitted, Commit, ShelvedFrom, SkillShelved,
};
use kernel::{Address, AxCode, AxError, EventDraft, EventKind, EventRef};
use kernel::{Ledger, Payload, RunId};
use storage::JsonlLedger;

use super::freezing::Assembled;
use super::{Hands, RunWorker, ScanReport};

#[derive(Debug)]
pub struct InitReport {
    pub ledger_dir: PathBuf,
    pub genesis: EventRef,

    pub standing: city::Standing,

    pub adopted: Vec<Address>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Adopt {
    Nothing,
    EveryFolder,
}

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

    let shipped = shelve_shipped(city_root)?;
    let now = crate::Clock::now(&*hands.clock)?;
    let (mut ledger, report) =
        JsonlLedger::open(&dir, now).map_err(storage::StorageError::into_ax)?;
    let genesis = ledger.append(EventDraft {
        run: RunId::CITY,
        t: now,
        who: "city".to_owned(),

        addr: kernel::layout::CityLayout::new(city_root).city_address(),
        kind: EventKind::CityInitialized,
        data: Payload::of(&kernel::event::record::CityInitialized {})?,
        ig: false,
    })?;
    record_shelved(&mut ledger, now, &shipped)?;

    city::ignore_city_records(city_root)?;

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

    let mut worker = RunWorker::over(
        city_root,
        runtime::diagnostics::Diagnostics::off(),
        hands,
        (ledger, report),
    )?;
    let plan = city::CityPlan::new(None)?;
    let (hall, template) = plan.hall();
    worker.create_building(hall.clone(), template.name())?;
    let mut adopted = Vec::new();
    if let (Adopt::EveryFolder, city::Standing::Work { adoptable, .. }) = (adopt, &standing) {
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

fn shelve_shipped(city_root: &Path) -> Result<Vec<city::Installed>, AxError> {
    let mut cas = storage::Cas::open(&kernel::layout::CityLayout::new(city_root).cas())
        .map_err(storage::StorageError::into_ax)?;
    city::shelve_shipped(city_root, &mut |bytes| {
        cas.put(bytes).map_err(storage::StorageError::into_ax)
    })
}

fn record_shelved(
    ledger: &mut JsonlLedger,
    now: kernel::TimeMs,
    shipped: &[city::Installed],
) -> Result<(), AxError> {
    for installed in shipped {
        match installed.placed {
            city::Placed::Fresh => {}
            city::Placed::AlreadyShelved => continue,
        }
        let shelved = SkillShelved {
            skill: installed.holding.name.clone(),
            digest: installed.hash,
            source: ShelvedFrom::Shipped,
        };
        ledger.append(EventDraft {
            run: RunId::CITY,
            t: now,
            who: "city".to_owned(),
            addr: None,
            kind: EventKind::SkillShelved,
            data: Payload::of(&shelved)?,
            ig: false,
        })?;
    }
    Ok(())
}

pub(super) fn city_segment(city_root: &Path) -> Result<Assembled, AxError> {
    prompt_segment(
        &city_root.join(city::CITY_FILE),
        Address::parse(city::CITY_FILE)?,
    )
}

pub(super) fn prompt_segment(path: &Path, at: Address) -> Result<Assembled, AxError> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Assembled::of_one(at, bytes)),
        Err(err) => Err(AxError::failure(
            AxCode::StorageFatal,
            "read the city's norms",
            format!("{}: {err}", path.display()),
        )
        .with_recovery(
            "every run under that file is governed by it; make it readable, or clear its \
             place and open the city again, which lays its default down",
        )),
    }
}

impl RunWorker {
    pub(super) fn create_building(&mut self, addr: Address, template: &str) -> Result<(), AxError> {
        let template = city::BuildingTemplate::parse(template)?;
        let building = city::create_building(&self.city_root, &addr, template)?;
        self.built(&building, template)
    }

    pub(super) fn raise_building(
        &mut self,
        addr: &Address,
        template: &str,
        rules: Option<&str>,
        source: Option<&std::path::Path>,
    ) -> Result<(), AxError> {
        let template = city::BuildingTemplate::parse(template)?;
        if let Some(text) = rules {
            let evaluated = city::evaluate(addr, text)?;
            self.validate_workspace_models(&evaluated)?;
        }
        let written = rules.map_or(city::RulesText::Template, city::RulesText::Written);
        let (building, founding) = city::raise_building(
            &self.city_root,
            city::Raising {
                addr,
                template,
                rules: written,
                source,
            },
        )?;
        match founding {
            city::Founding::Built => self.built(&building, template),
            city::Founding::Adopted => self.adopted(&building, template),
        }
    }

    fn built(
        &mut self,
        building: &city::Building,
        template: city::BuildingTemplate,
    ) -> Result<(), AxError> {
        self.note(
            runtime::diagnostics::Level::Effect,
            "city::building",
            &format!(
                "{} laid out from the {} template",
                building.addr().as_str(),
                template.name()
            ),
        );
        let payload = city::building_created_payload(building, template)?;
        self.record(EventKind::BuildingCreated, payload)
    }

    pub fn adopt_building(&mut self, addr: Address) -> Result<(), AxError> {
        let building = city::adopt_building(&self.city_root, &addr)?;
        self.adopted(&building, city::BuildingTemplate::Minimal)
    }

    fn adopted(
        &mut self,
        building: &city::Building,
        template: city::BuildingTemplate,
    ) -> Result<(), AxError> {
        self.note(
            runtime::diagnostics::Level::Effect,
            "city::building",
            &format!("{} adopted as a building", building.addr().as_str()),
        );
        let base = match self.adoption_base(building) {
            Ok(base) => base,
            Err(refused) => return Err(self.withdrawn(building, refused)),
        };
        let payload = city::building_adopted_payload(building, template)?;
        self.record(EventKind::BuildingCreated, payload)?;

        if let Some(base) = base {
            self.record_at(
                EventKind::CheckpointCommitted,
                building.addr().clone(),
                Payload::of(&CheckpointCommitted::Committed(base))?,
            )?;
        }
        Ok(())
    }

    fn withdrawn(&self, building: &city::Building, refused: AxError) -> AxError {
        match city::withdraw_adoption(&self.city_root, building) {
            Ok(()) => refused,
            Err(left) => {
                let recovery = format!(
                    "{}; the rules this adoption wrote are still there ({}): delete that file,                      then adopt again",
                    refused.recovery(),
                    left.subject()
                );
                refused.rewrite_recovery(recovery)
            }
        }
    }

    fn adoption_base(&mut self, building: &city::Building) -> Result<Option<Commit>, AxError> {
        let addr = building.addr();

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
        let entropy_allow = city::load(&self.city_root, addr)?
            .checkpoint_entropy_allow()
            .clone();
        let made = storage::Checkpoint::open(&self.city_root).and_then(|checkpoint| {
            checkpoint
                .with_entropy_allowlist(entropy_allow)
                .base_checkpoint(&[addr.as_str().to_owned()], t, &of, &mut |step| {
                    self.note(
                        runtime::diagnostics::Level::Effect,
                        "storage::checkpoint",
                        &format!("{}: base checkpoint {step:?}", addr.as_str()),
                    );
                })
        });
        match made {
            Ok(record) => Ok(Some(record)),
            Err(err) => {
                let failure = runtime::CheckpointFailure::from(err);
                if let Some(report) = failure.refusal() {
                    self.record_at(
                        EventKind::SecretEgressBlocked,
                        addr.clone(),
                        Payload::of(report)?,
                    )?;
                }
                Err(failure.into_cause())
            }
        }
    }

    pub fn startup_scan(&mut self) -> Result<ScanReport, AxError> {
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

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod shipped_tests;

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod startup_tests;
