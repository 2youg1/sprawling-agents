// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use kernel::event::record::PursuitChanged;
use kernel::{Address, AxError, EventKind, EventRecord};

use super::kinds::Holding;
use super::lines::{discard_lines, pursued, registry_line, restored_paths, signal_line};
use super::snapshot::start::city_root_of;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Views {
    #[serde(skip)]
    pub(super) city_root: PathBuf,
    pub(super) hot: storage::HotView,
    pub(super) attribution: storage::Attribution,

    pub(super) governance: super::Governance,
    pub(super) book: gateway::EndpointBook,

    pub(super) city: Option<Address>,

    pub(super) head: Option<kernel::Seq>,

    pub(super) epoch: Option<kernel::B3Hash>,

    pub(super) waiting: std::collections::BTreeMap<Address, Vec<wire::SignalLine>>,

    pub(super) discards: std::collections::BTreeMap<String, wire::DiscardLine>,

    pub(super) assets: Vec<wire::RegistryLine>,

    pub(super) commits: std::collections::BTreeMap<kernel::GitOid, super::commits::CommitFacts>,

    pub(super) commit_seqs: std::collections::BTreeMap<kernel::Seq, kernel::GitOid>,

    pub(super) offer_times: super::proposals::OfferTimes,

    pub(super) predecessors: std::collections::BTreeMap<kernel::RunId, kernel::RunId>,

    pub(super) last_commit: std::collections::BTreeMap<kernel::RunId, wire::CommitAt>,

    pub(super) skill_pins: std::collections::BTreeMap<(String, kernel::B3Hash), Vec<kernel::RunId>>,

    pub(super) events: u64,

    pub(super) next_unfolded: kernel::Seq,
    #[serde(
        serialize_with = "super::snapshot::encode_index",
        deserialize_with = "super::snapshot::decode_index"
    )]

    pub(super) index: std::sync::Arc<std::sync::Mutex<storage::LedgerIndex>>,

    pub(super) first_prompts: std::collections::BTreeMap<kernel::RunId, kernel::Seq>,

    pub(super) latest_shapes: std::collections::BTreeMap<kernel::RunId, kernel::Seq>,

    pub(super) building_prompts: std::collections::BTreeMap<Address, RunPrompt>,

    pub(super) unprompted: std::collections::BTreeMap<kernel::RunId, Started>,

    pub(super) sessions: super::sessions::RoomSessions,

    pub(super) saves: super::versions::DocumentSaves,

    #[serde(
        serialize_with = "super::snapshot::encode_plans",
        deserialize_with = "super::snapshot::decode_plans"
    )]
    pub(super) plans: std::sync::Arc<std::sync::Mutex<crate::plan_view::PlanView>>,

    pub(super) pursuits: std::collections::BTreeMap<Address, (String, kernel::PursuitState)>,

    pub(super) decided: Vec<wire::Decision>,

    pub(super) claims:
        std::collections::BTreeMap<kernel::NodeId, std::collections::BTreeSet<kernel::RunId>>,

    #[serde(skip)]
    pub(super) machine: Option<wire::DoctorAnswer>,

    #[serde(skip)]
    pub(super) vault: Option<std::sync::Arc<std::sync::Mutex<gateway::Custodian>>>,

    #[serde(skip)]
    pub(super) reach: super::served::Reach,

    #[serde(skip)]
    pub(super) offered: Option<crate::offered::Offered>,

    #[serde(skip)]
    pub(super) proof: Option<storage::ChainHalt>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct Started {
    pub(super) room: Address,
    pub(super) at: kernel::Seq,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct RunPrompt {
    pub(super) started: kernel::Seq,
    pub(super) prompt: kernel::Seq,
}

impl Views {

    pub fn over(ledger_dir: &Path) -> Views {
        Views::new(city_root_of(ledger_dir))
    }

    pub fn new(city_root: &Path) -> Views {

        Views::sharing(
            city_root,
            super::snapshot::fresh_index(),
            std::sync::Arc::default(),
        )
    }

    fn sharing(
        city_root: &Path,
        index: std::sync::Arc<std::sync::Mutex<storage::LedgerIndex>>,
        plans: std::sync::Arc<std::sync::Mutex<crate::plan_view::PlanView>>,
    ) -> Views {
        Views {
            city_root: city_root.to_path_buf(),
            hot: storage::HotView::new(),
            attribution: storage::Attribution::new(),
            governance: super::Governance::empty(),
            book: gateway::EndpointBook::new(),
            city: None,
            head: None,
            epoch: None,
            waiting: std::collections::BTreeMap::new(),
            discards: std::collections::BTreeMap::new(),
            assets: Vec::new(),
            commits: std::collections::BTreeMap::new(),
            commit_seqs: std::collections::BTreeMap::new(),
            offer_times: super::proposals::OfferTimes::default(),
            predecessors: std::collections::BTreeMap::new(),
            last_commit: std::collections::BTreeMap::new(),
            skill_pins: std::collections::BTreeMap::new(),
            events: 0,
            next_unfolded: kernel::Seq::FIRST,
            index,
            first_prompts: std::collections::BTreeMap::new(),
            latest_shapes: std::collections::BTreeMap::new(),
            building_prompts: std::collections::BTreeMap::new(),
            unprompted: std::collections::BTreeMap::new(),
            sessions: super::sessions::RoomSessions::default(),
            saves: super::versions::DocumentSaves::default(),
            plans,
            pursuits: std::collections::BTreeMap::new(),
            decided: Vec::new(),
            claims: std::collections::BTreeMap::new(),
            machine: None,
            vault: None,
            reach: super::served::Reach::default(),
            offered: None,
            proof: None,
        }
    }

    pub(crate) fn hold_index(
        &mut self,
        index: storage::LedgerIndex,
        ledger_dir: &Path,
    ) -> Result<(), AxError> {
        self.epoch = match index.reader(ledger_dir).line_at(kernel::Seq::FIRST) {
            Ok(genesis) => Some(kernel::ledger::chain_hash(&genesis)),
            Err(storage::StorageError::SeqMissing { .. }) => None,
            Err(other) => return Err(other.into_ax()),
        };

        *self
            .index
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = index;
        self.index.clear_poison();
        Ok(())
    }

    pub(crate) fn next_unfolded(&self) -> kernel::Seq {
        self.next_unfolded
    }

    pub fn apply(&mut self, record: &EventRecord) -> Result<(), AxError> {

        let next_unfolded = record.seq().next()?;
        self.head = Some(record.seq());
        self.hot
            .apply(record)
            .map_err(storage::StorageError::into_ax)?;
        self.attribution
            .apply(record)
            .map_err(storage::StorageError::into_ax)?;

        if record.kind() == EventKind::RunFrozen || self.hot.was_evicted(&record.run()) {
            let hot = &self.hot;
            self.attribution.retain_runs(|run| !hot.was_evicted(run));
        }
        crate::listed_facts::apply(&mut self.book, &self.city_root, record)?;

        self.governance
            .absorb(record.kind(), record.run(), record.addr(), record.data())?;
        self.offer_times.absorb(record)?;
        self.sessions.absorb(record)?;
        crate::plan_view::PlanView::take_back(&self.plans).apply(record);
        self.events = self.events.saturating_add(1);
        self.next_unfolded = next_unfolded;
        match Holding::of(record.kind()) {
            Holding::City => {
                self.city = record.addr().cloned();
            }
            Holding::SignalQueued => {
                let (room, line) = signal_line(record)?;
                self.waiting.entry(room).or_default().push(line);
            }
            Holding::SignalTaken => {
                let taken = record
                    .data()
                    .read::<kernel::event::record::SignalConsumed>()?;
                for queue in self.waiting.values_mut() {
                    queue.retain(|held| held.id != taken.id.as_str());
                }
            }
            Holding::Pursuit => {
                let addr = pursued(record)?;
                match record.data().read::<PursuitChanged>()?.held()? {
                    Some(entry) => {
                        self.pursuits.insert(addr, entry);
                    }
                    None => {
                        self.pursuits.remove(&addr);
                    }
                }
            }
            Holding::Discarded => {
                for line in discard_lines(record) {
                    self.discards.insert(line.path.clone(), line);
                }
            }
            Holding::Restored => {
                for path in restored_paths(record) {
                    if let Some(held) = self.discards.get_mut(&path) {
                        held.restored = true;
                    }
                }
            }
            Holding::Claimed => {

                let node = record
                    .data()
                    .read::<kernel::event::record::RoadmapMoved>()?
                    .node;
                self.claims.entry(node).or_default().insert(record.run());
            }
            Holding::Commit => self.fold_commit(record)?,
            Holding::RunStarted => {
                self.fold_predecessor(record);
                self.fold_skill_pins(record);
                if let Some(addr) = record.addr() {
                    self.unprompted.insert(
                        record.run(),
                        Started {
                            room: addr.clone(),
                            at: record.seq(),
                        },
                    );
                }
            }
            Holding::Asset => {
                if let Some(line) = registry_line(record) {
                    self.assets.push(line);
                }
            }
            Holding::Ruling => self.fold_ruling(record)?,
            Holding::Document => self.saves.absorb(record)?,
            Holding::Prompt => {
                if let std::collections::btree_map::Entry::Vacant(first) =
                    self.first_prompts.entry(record.run())
                {
                    first.insert(record.seq());
                }

                if let Some(Started { room, at: started }) = self.unprompted.remove(&record.run())
                    && let Ok(building) = city::Building::of(&room)
                {
                    let folded = RunPrompt {
                        started,
                        prompt: record.seq(),
                    };
                    self.building_prompts
                        .entry(building.addr().clone())
                        .and_modify(|held| {
                            if held.started < started {
                                *held = folded;
                            }
                        })
                        .or_insert(folded);
                }
            }
            Holding::PromptShape => {
                self.latest_shapes.insert(record.run(), record.seq());
            }
            Holding::Nothing => {}
        }
        Ok(())
    }

    pub fn epoch(&self) -> Option<kernel::B3Hash> {
        self.epoch
    }

    pub fn head(&self) -> Option<kernel::Seq> {
        self.head
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, reason = "test code")]
mod tests {
    use kernel::{Address, B3Hash, EventKind, RunId, Seq};

    use super::Views;
    use crate::views::tests::{Place, view_record};

    #[test]
    fn an_answer_contains_genesis_exactly_when_something_was_folded() {
        let dir = tempfile::tempdir().unwrap();
        let mut views = Views::new(dir.path());
        let contains_genesis = |views: &Views| Seq::FIRST.value() < views.next_unfolded().value();
        let before = contains_genesis(&views);
        let serde_json::Value::Object(data) = serde_json::json!({
            "at": "lab/Memo.md",
            "baseline": B3Hash::digest(b"zero"),
            "version": B3Hash::digest(b"one"),
            "bytes": 3,
        }) else {
            panic!("an object");
        };
        views
            .apply(&view_record(
                Place {
                    seq: 0,
                    run: RunId::CITY,
                },
                EventKind::DocumentWritten,
                &Address::parse("lab/Memo.md").unwrap(),
                data,
            ))
            .unwrap();
        assert_eq!((before, contains_genesis(&views)), (false, true));
    }
}
