// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod closing;

pub use closing::{ClosedBy, Closing};

use super::{
    Collaborating, Credentials, Doorstep, Flight, GatewayModels, Hands, Planning, RoomQueues,
    RunWorker, Standing,
};
use std::path::Path;

use kernel::event::record::{ShelvedFrom, SkillAudited, SkillShelved};
use kernel::{AxError, B3Hash, EventDraft, EventKind, EventRecord, Ledger, Payload, RunId, TimeMs};
use storage::{Cas, JsonlLedger, OpenReport};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LedgerOpening {
    Intact,

    TailDropped { bytes: u64 },
}

impl From<OpenReport> for LedgerOpening {
    fn from(report: OpenReport) -> Self {
        match report.recovered {
            None => LedgerOpening::Intact,
            Some(cut) => LedgerOpening::TailDropped {
                bytes: cut.dropped_bytes,
            },
        }
    }
}

impl LedgerOpening {
    pub fn notice(self) -> Option<String> {
        match self {
            LedgerOpening::Intact => None,
            LedgerOpening::TailDropped { bytes } => Some(format!(
                "the ledger's last line was half-written when the city last stopped, so \
                 opening cut {bytes} byte(s) from its tail and recorded log_truncated; every \
                 earlier line verified. Check whether the last action before that stop needs \
                 doing again"
            )),
        }
    }
}

fn old_skill_bindings(
    city_root: &Path,
) -> Result<std::collections::BTreeMap<String, (ShelvedFrom, Vec<SkillAudited>)>, AxError> {
    let lines = storage::read_raw_lines_at(&kernel::layout::CityLayout::new(city_root).ledger())
        .map_err(storage::StorageError::into_ax)?;
    let mut bindings = std::collections::BTreeMap::new();
    for line in lines {
        let record = EventRecord::parse_line(&line)?;
        if record.kind() == EventKind::SkillShelved {
            let shelved = record.data().read::<SkillShelved>()?;
            let entry = bindings
                .entry(shelved.skill)
                .or_insert_with(|| (ShelvedFrom::Path, Vec::new()));
            entry.0 = shelved.source;
        } else if record.kind() == EventKind::SkillAudited {
            let audit = record.data().read::<SkillAudited>()?;
            let entry = bindings
                .entry(audit.skill.clone())
                .or_insert_with(|| (ShelvedFrom::Path, Vec::new()));
            entry.1.push(audit);
        }
    }
    Ok(bindings)
}

impl RunWorker {
    pub fn new(
        city_root: &Path,
        log: runtime::diagnostics::Diagnostics,
        hands: Hands,
    ) -> Result<Self, AxError> {
        let opened = JsonlLedger::open(
            &kernel::layout::CityLayout::new(city_root).ledger(),
            crate::Clock::now(&*hands.clock)?,
        )
        .map_err(storage::StorageError::into_ax)?;
        RunWorker::over(city_root, log, hands, opened)
    }

    pub fn over(
        city_root: &Path,
        log: runtime::diagnostics::Diagnostics,
        hands: Hands,
        (ledger, report): (JsonlLedger, OpenReport),
    ) -> Result<Self, AxError> {
        let standing = Standing::fold(&kernel::layout::CityLayout::new(city_root).ledger())?;
        RunWorker::holding(city_root, log, hands, (ledger, report, standing))
    }

    pub fn holding(
        city_root: &Path,
        mut log: runtime::diagnostics::Diagnostics,
        hands: Hands,
        (ledger, report, standing): (JsonlLedger, OpenReport, Standing),
    ) -> Result<Self, AxError> {
        let Hands {
            vault,
            clock,
            monotonic,
            machine,
            read_memory,
            read_volume,
            reveal,
            browsers,
            desktop_program,
            recipe_for,
            exec_host,
            seat_lane,
            shares,
            affinity,
            budget,
        } = hands;
        let now = crate::Clock::now(&*clock)?;

        city::keep_records_out_of_git(city_root)?;

        for edited in city::follow_defaults(city_root)? {
            log.write(
                runtime::diagnostics::Level::Effect,
                runtime::diagnostics::Site {
                    run: kernel::RunId::CITY,
                    seq: ledger.position(),
                    module: "accounting::worker",
                },
                &format!(
                    "{} differs from every default a build of sprawling laid down, so it is kept as written and does not follow this build's default; delete it to take the default",
                    edited.path.display()
                ),
            );
        }
        let Standing {
            book,
            governance,
            collaboration,
            entrance,
            origins,
            cut,
        } = standing;
        if let Err(fault) = cut {
            log.write(
                runtime::diagnostics::Level::Refuse,
                runtime::diagnostics::Site {
                    run: kernel::RunId::CITY,
                    seq: ledger.position(),
                    module: "accounting::worker",
                },
                &format!(
                    "the standing snapshot was not cut: {fault}; the city goes on, and the next start folds from the older snapshot or from genesis"
                ),
            );
        }
        let cas = Cas::open(&kernel::layout::CityLayout::new(city_root).cas())
            .map_err(storage::StorageError::into_ax)?;
        let book = crate::listed_facts::learned(book, &cas)?;
        let lane_store = std::sync::Arc::new(std::sync::Mutex::new(
            Cas::open(&kernel::layout::CityLayout::new(city_root).cas())
                .map_err(storage::StorageError::into_ax)?,
        ));

        let delegator = kernel::Delegator::root();
        let pursuits = collaboration.pursuits(&delegator);
        let mut worker = RunWorker {
            city_root: city_root.to_path_buf(),
            city: std::sync::OnceLock::new(),
            recovered_projects: std::collections::BTreeSet::new(),
            ledger,
            opening: LedgerOpening::from(report),
            cas,
            lane_store,
            credentials: Credentials::opened(book, vault),
            serving: None,
            governance,
            collaborating: Collaborating {
                rooms: RoomQueues::folded(collaboration.inboxes),
                joins: collaboration.joins,
                workshops: std::collections::BTreeMap::new(),
                handing: std::collections::BTreeMap::new(),
                requests: collaboration.requests,
                goals: collaboration.goals,
            },
            planning: Planning {
                pursuits,
                delegator,
                holders: collaboration.plan_holders,
                write_plan: city::edit_against,
            },
            last_tick: now,
            log: super::recording::Notes::over(log),
            doorstep: Doorstep::opened(entrance),
            origins,
            flight: Flight::open(
                read_memory,
                monotonic,
                seat_lane,
                runtime::Backlog::new()
                    .with_shares(shares)
                    .with_affinity(affinity)
                    .with_budget(budget),
            ),
            index: storage::LedgerIndex::empty(),
            warm: super::keeping_warm::Kept::default(),
            models: Box::new(GatewayModels { monotonic }),
            connectors: std::sync::Arc::new(super::mcp::Residents::default()),
            drift_book: super::workbench::DriftBook::from_history(city_root)?,
            machine,
            clock,
            monotonic,
            read_volume,
            reveal,
            browsers,
            desktop_program,
            harnesses: super::driving::harness::on_this_machine(),
            recipe_for,
            exec_host,
        };
        worker.migrate_skills(now)?;
        for building in city::buildings(city_root)? {
            city::migrate_mcp(city_root, &building)?;
        }
        worker.bring_configuration_forward();
        Ok(worker)
    }

    fn migrate_skills(&mut self, now: TimeMs) -> Result<(), AxError> {
        let store = kernel::layout::CityLayout::new(&self.city_root).cas();
        let mut cas = Cas::open(&store).map_err(storage::StorageError::into_ax)?;
        let mut migrated = {
            let reader = Cas::open(&store).map_err(storage::StorageError::into_ax)?;
            let mut register =
                |bytes: &[u8]| cas.put(bytes).map_err(storage::StorageError::into_ax);
            let verify =
                |digest: &B3Hash| reader.get(digest).map_err(storage::StorageError::into_ax);
            city::Library::migrate(
                &self.city_root,
                None,
                city::MigrationHooks {
                    register: &mut register,
                    verify: &verify,
                },
            )?
        };
        for building in city::buildings(&self.city_root)? {
            migrated.extend({
                let reader = Cas::open(&store).map_err(storage::StorageError::into_ax)?;
                let mut register =
                    |bytes: &[u8]| cas.put(bytes).map_err(storage::StorageError::into_ax);
                let verify =
                    |digest: &B3Hash| reader.get(digest).map_err(storage::StorageError::into_ax);
                city::Library::migrate(
                    &self.city_root,
                    Some(&building),
                    city::MigrationHooks {
                        register: &mut register,
                        verify: &verify,
                    },
                )?
            });
        }
        if migrated.is_empty() {
            return Ok(());
        }
        let mut bindings = old_skill_bindings(&self.city_root)?;
        for installed in migrated {
            let Some(previous) = installed.previous else {
                continue;
            };
            let (source, audits) = bindings
                .remove(&installed.holding.name)
                .unwrap_or_else(|| (ShelvedFrom::Path, Vec::new()));
            self.ledger.append(EventDraft {
                run: RunId::CITY,
                t: now,
                who: "city".to_owned(),
                addr: None,
                kind: EventKind::SkillShelved,
                data: Payload::of(&SkillShelved {
                    skill: installed.holding.name.clone(),
                    digest: installed.hash,
                    source,
                })?,
                ig: false,
            })?;
            for audit in audits.into_iter().filter(|audit| audit.digest == previous) {
                self.ledger.append(EventDraft {
                    run: RunId::CITY,
                    t: now,
                    who: "city".to_owned(),
                    addr: None,
                    kind: EventKind::SkillAudited,
                    data: Payload::of(&SkillAudited {
                        digest: installed.hash,
                        ..audit
                    })?,
                    ig: false,
                })?;
            }
        }
        Ok(())
    }

    pub(super) fn recover_project(&mut self, building: &kernel::Address) -> Result<(), AxError> {
        if self.recovered_projects.contains(building) {
            return Ok(());
        }
        let project = storage::ProjectRepository::at(&self.city_root, building)
            .map_err(storage::StorageError::into_ax)?;
        if project.root().join(".git").exists() {
            self.reconcile_project(
                &project,
                &project.trees().map_err(storage::StorageError::into_ax)?,
            )?;
        }
        storage::Worktrees::lift_abandoned_leases(project.root(), &self.ledger)
            .map_err(storage::StorageError::into_ax)?;
        storage::Worktrees::sweep_abandoned(project.root(), &[])
            .map_err(storage::StorageError::into_ax)?;
        self.recovered_projects.insert(building.clone());
        Ok(())
    }

    fn bring_configuration_forward(&mut self) {
        let told: Vec<String> = match crate::home::Home::detect()
            .and_then(|home| city::bring_city_forward(&self.city_root, &home.backups()))
        {
            Ok(files) => files
                .into_iter()
                .map(|forwarding| match forwarding.outcome {
                    Ok(city::Forwarded::Moved { from, to, backup }) => format!(
                        "brought {} from format {} to {}; the original is at {}",
                        forwarding.path.display(),
                        from.get(),
                        to.get(),
                        backup.display()
                    ),
                    Ok(city::Forwarded::Absent | city::Forwarded::Current) => format!(
                        "{} needed nothing brought forward",
                        forwarding.path.display()
                    ),
                    Err(err) => format!(
                        "could not bring {} forward: {err}",
                        forwarding.path.display()
                    ),
                })
                .collect(),
            Err(err) => vec![format!(
                "could not bring the city's configuration files forward; the next open tries again: {err}"
            )],
        };
        for line in told {
            self.note(
                runtime::diagnostics::Level::Effect,
                "accounting::worker",
                &line,
            );
        }
    }

    pub fn take_back_the_lost(&mut self) {
        self.await_proof();
        let (level, told) = match self.startup_scan() {
            Ok(report) if report.frozen_runs == 0 && report.closed_calls == 0 => return,
            Ok(report) => (
                runtime::diagnostics::Level::Effect,
                format!(
                    "froze {} run(s) the last process left open, after closing {} call(s) whose outcome it left unknown",
                    report.frozen_runs, report.closed_calls
                ),
            ),
            Err(err) => (
                runtime::diagnostics::Level::Refuse,
                format!(
                    "the runs the last process left open were not frozen: {err}; {}",
                    err.recovery()
                ),
            ),
        };
        self.note(level, "accounting::worker", &told);
    }

    pub fn opening(&self) -> LedgerOpening {
        self.opening
    }

    #[must_use]
    pub fn with_clock(self, clock: std::sync::Arc<dyn crate::Clock + Send + Sync>) -> RunWorker {
        RunWorker { clock, ..self }
    }

    #[must_use]
    pub fn with_browsers(self, browsers: super::Browsers) -> RunWorker {
        RunWorker { browsers, ..self }
    }

    #[must_use]
    pub fn with_desktop_program(self, program: super::DesktopProgram) -> RunWorker {
        RunWorker {
            desktop_program: program,
            ..self
        }
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn with_harnesses(self, start: super::driving::harness::StartHarness) -> RunWorker {
        RunWorker {
            harnesses: start,
            ..self
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests;
