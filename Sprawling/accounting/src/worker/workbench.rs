// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use kernel::{Address, AxCode, AxError, RunId};
use runtime::bench::ToolBench;

use kernel::event::record::autonomy_word;

mod desks;
pub(in crate::worker) use desks::{BenchDesks, Desks};
mod desktop;
mod engine;
mod servers;
mod standing;
mod tools;

pub(super) use standing::{Lending, Placing, lend_tree};

pub(crate) use tools::catalog_builtins;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct LocatedDrift {
    pub(super) building: Address,
    pub(super) drift: kernel::event::record::CatalogDrift,
}

#[derive(Clone, Default)]
pub(super) struct DriftBook {
    seen: std::sync::Arc<std::sync::Mutex<std::collections::BTreeSet<LocatedDrift>>>,
}

impl DriftBook {

    pub(super) fn from_history(city_root: &Path) -> Result<DriftBook, AxError> {
        let book = DriftBook::default();
        for line in storage::read_raw_lines_at(&kernel::layout::CityLayout::new(city_root).ledger())
            .map_err(storage::StorageError::into_ax)?
        {
            let record = kernel::EventRecord::parse_line(&line)?;
            if record.kind() != kernel::EventKind::DigestInvalidated {
                continue;
            }
            let drift = record
                .data()
                .read::<kernel::event::record::CatalogDrift>()?;
            if let Some(building) = record.addr() {
                book.seen
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .insert(LocatedDrift {
                        building: building.clone(),
                        drift,
                    });
            }
        }
        Ok(book)
    }

    pub(super) fn record(
        &self,
        next: &LocatedDrift,
        write: impl FnOnce() -> Result<(), AxError>,
    ) -> Result<(), AxError> {
        let mut seen = held(&self.seen, "record catalog drift")?;
        if seen.contains(next) {
            return Ok(());
        }
        write()?;
        seen.insert(next.clone());
        Ok(())
    }
}

pub(in crate::worker) struct ConnectorJob<'a> {
    pub(in crate::worker) servers: &'a [kernel::McpServer],
    pub(in crate::worker) references: &'a [kernel::McpReference],
    pub(in crate::worker) building: &'a Address,

    pub(in crate::worker) session: &'a Address,
    pub(in crate::worker) write_root: &'a Path,
    pub(in crate::worker) confidential: bool,
}

pub(in crate::worker) struct Laying {
    pub(in crate::worker) city_root: PathBuf,

    pub(in crate::worker) city: kernel::B3Hash,
    vault: std::sync::Arc<std::sync::Mutex<gateway::Custodian>>,
    connectors: std::sync::Arc<dyn crate::Connectors + Send + Sync>,

    browsers: super::Browsers,

    desktop_program: super::DesktopProgram,
    exec_host: super::hands::ExecHost,
    backlog: runtime::Backlog,

    edits: runtime::PathLocks,

    pub(in crate::worker) store: std::sync::Arc<std::sync::Mutex<storage::Cas>>,
    pub(in crate::worker) notes: super::recording::Notes,

    clock: std::sync::Arc<dyn crate::Clock + Send + Sync>,

    monotonic: fn() -> std::time::Instant,

    pub(in crate::worker) staged_at: kernel::Seq,
    trust: kernel::Autonomy,

    waiting: std::collections::BTreeMap<Address, u32>,

    locks: Vec<String>,

    book: gateway::EndpointBook,
    proposing: tools::proposal::Proposing,

    drift: std::sync::Mutex<Vec<LocatedDrift>>,

    pub(super) drift_book: DriftBook,
}

impl Laying {

    pub(super) fn note_drift(
        &self,
        building: &Address,
        drift: kernel::event::record::CatalogDrift,
    ) -> Result<(), AxError> {
        held(&self.drift, "collect catalog drift")?.push(LocatedDrift {
            building: building.clone(),
            drift,
        });
        Ok(())
    }

    pub(super) fn take_drift(&self) -> Result<Vec<LocatedDrift>, AxError> {
        Ok(std::mem::take(&mut *held(
            &self.drift,
            "collect catalog drift",
        )?))
    }

    fn note(&self, level: runtime::diagnostics::Level, module: &str, message: &str) {
        self.notes.write(level, self.staged_at, module, message);
    }

    fn open_checkpoint(
        &self,
        root: &std::path::Path,
        writer: kernel::RunId,
        entropy_allow: kernel::EntropyAllowlist,
    ) -> Result<storage::Checkpoint, AxError> {
        storage::Checkpoint::open_writer(root, writer)
            .map(|checkpoint| checkpoint.with_entropy_allowlist(entropy_allow))
            .map_err(storage::StorageError::into_ax)
    }
}

impl super::RunWorker {

    pub(in crate::worker) fn laying(&self, who: &str) -> Result<Laying, AxError> {
        Ok(Laying {
            city_root: self.city_root.clone(),
            city: self.city_hash()?,
            vault: self.vault_handle(),
            connectors: std::sync::Arc::clone(&self.connectors),
            browsers: self.browsers,
            desktop_program: self.desktop_program,
            exec_host: self.exec_host,
            backlog: self.flight.backlog.clone(),
            edits: self.flight.edits.clone(),
            store: std::sync::Arc::clone(&self.lane_store),
            notes: self.log.clone(),
            clock: std::sync::Arc::clone(&self.clock),
            monotonic: self.monotonic,
            staged_at: self.ledger.position(),
            trust: self.governance.autonomy.clone(),
            waiting: self.collaborating.rooms.waiting(),
            locks: self
                .collaborating
                .goals
                .iter()
                .filter(|entry| entry.owner == who)
                .map(|entry| entry.statement.clone())
                .collect(),
            book: self.credentials.book.clone(),
            proposing: self.proposing(),
            drift: std::sync::Mutex::new(Vec::new()),
            drift_book: self.drift_book.clone(),
        })
    }
}

pub(super) const CITY_VERIFIER: &str = "city";

pub(super) struct Site {
    pub(super) building: city::Building,
    pub(super) rules: city::BuildingRules,

    pub(super) config: kernel::FrozenConfig,

    pub(super) reading_room: Vec<String>,
    pub(super) model: gateway::ModelEntry,

    pub(super) routing: kernel::event::record::ModelRouting,

    pub(super) generation: kernel::FrozenGeneration,

    pub(super) provider: String,
    pub(super) adapter: Option<super::keeping_warm::Door>,
    pub(super) identity: city::Identity,
    pub(super) who: String,
    pub(super) run_id: RunId,

    pub(super) predecessor: Option<RunId>,

    pub(super) lease: Option<storage::WorktreeLease>,
    pub(super) write_root: PathBuf,
    branch: Option<String>,

    pub(super) filters: runtime::FilterTable,

    pub(super) clock: runtime::ClockReading,

    pub(super) retries: kernel::Retries,
}

pub(super) struct Workbench {
    pub(super) catalog: std::sync::Arc<std::sync::Mutex<runtime::Catalog>>,

    pub(super) drift: Vec<LocatedDrift>,

    bench: Option<ToolBench>,

    policy: Option<runtime::PolicyCell>,

    pub(super) succession: std::sync::Arc<std::sync::Mutex<runtime::SuccessionDesk>>,

    pub(super) context: runtime::ContextReading,

    shown: runtime::ReadShown,

    rule_candidates: runtime::RuleCandidates,
}

pub(super) struct Reach<'a> {
    pub(super) seen: &'a city::Neighbourhood,
    pub(super) delegates: &'a std::sync::Arc<std::sync::Mutex<collab::DelegateDesk>>,
    pub(super) context: &'a runtime::ContextReading,

    pub(super) catalog: &'a std::sync::Arc<std::sync::Mutex<runtime::Catalog>>,
}

pub(super) fn held<'a, T>(
    desk: &'a std::sync::Mutex<T>,
    what: &'static str,
) -> Result<std::sync::MutexGuard<'a, T>, AxError> {
    desk.lock().map_err(|_| {
        AxError::failure(
            AxCode::StorageFatal,
            what,
            "the desk was left locked by a thread that died",
        )
        .with_recovery("restart this city")
    })
}

impl Workbench {

    pub(super) fn shown(&self) -> runtime::ReadShown {
        self.shown.clone()
    }

    pub(super) fn rule_candidates(&self) -> runtime::RuleCandidates {
        self.rule_candidates.clone()
    }

    pub(super) fn take_bench(&mut self) -> Result<ToolBench, AxError> {
        self.bench.take().ok_or_else(|| {
            AxError::failure(
                AxCode::ConfigInvalid,
                "carry the bench into the drive",
                "this workbench has already been driven",
            )
            .with_recovery("report this: one dispatch lays out one bench and drives it once")
        })
    }

    pub(super) fn take_policy(&mut self) -> Result<runtime::PolicyCell, AxError> {
        self.policy.take().ok_or_else(|| {
            AxError::failure(
                AxCode::ConfigInvalid,
                "carry the run policy into the plan",
                "this workbench's policy cell was already taken",
            )
            .with_recovery("report this: one dispatch freezes one plan")
        })
    }
}

impl Site {

    pub(super) fn checkpoint_scope(&self) -> Result<Vec<String>, AxError> {
        if self.lease.is_some() {
            return Ok(tree_scope(&self.building));
        }
        Ok(self
            .rules
            .write_domain()?
            .prefixes()
            .map(|prefix| prefix.as_str().to_owned())
            .collect())
    }
}

pub(in crate::worker) fn tree_scope(building: &city::Building) -> Vec<String> {
    vec![building.addr().as_str().to_owned()]
}

pub(super) struct Situation<'a> {
    pub(super) addr: &'a Address,
    pub(super) who: &'a str,
    signals_pending: u32,
    policy: kernel::RunPolicy,
    write_domain: &'a kernel::WriteDomain,
    worktree: &'a Path,
    trust: &'a kernel::Autonomy,
    context_tokens: u64,
    locks: Vec<String>,
    neighbours: u32,
}

pub(super) fn status_snapshot(situation: Situation<'_>) -> runtime::StatusSnapshot {
    runtime::StatusSnapshot {
        who: situation.who.to_owned(),
        addr: situation.addr.clone(),
        policy: situation.policy,
        ctx_limit: kernel::Tokens::new(situation.context_tokens),
        trust: autonomy_word::spell(situation.trust),
        write_domain: situation
            .write_domain
            .prefixes()
            .map(|prefix| prefix.as_str().to_owned())
            .collect::<Vec<String>>()
            .join(", "),
        locks: situation.locks,
        worktree: situation.worktree.to_path_buf(),
        signals_pending: situation.signals_pending,
        provider_mode: runtime::ProviderMode::Normal,
        neighbours: situation.neighbours,
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
