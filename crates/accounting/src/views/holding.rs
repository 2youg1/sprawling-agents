// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The fold every query is answered from, and the lines a page reads off
//! it.
//!
//! **Why it is a projection and not part of the assembly point.** Nothing
//! here decides anything or reaches a provider: it folds records into the
//! answers a client asks for, and `Views::rebuild` throws the whole thing
//! away and folds the ledger again to get the same bytes. That is
//! ARCHITECTURE.md section 9 shape 7, while `bin::assembly` is an
//! adapter - and a file holding two shapes is what section 9 says a split
//! looks like.
//!
//! **What it deliberately does not hold.** The plans are
//! `crate::plan_view`'s and are read through it; a second parse here
//! would be a second answer to "what is stuck and why", and only one of
//! them would be folding the records that say why. What waits in a room
//! is folded from signal records rather than read off a queue, because a
//! queue answers by being consumed and a view that consumed what it
//! showed would change the thing it reports on.

use std::path::{Path, PathBuf};

use kernel::event::record::PursuitChanged;
use kernel::{Address, AxError, EventKind, EventRecord};

use super::lines::{discard_lines, pursued, registry_line, restored_paths, signal_line};
use super::snapshot::start::city_root_of;

/// The derived views a query reads. They are rebuilt from the ledger at
/// startup and folded forward by the write observer, so deleting them
/// costs nothing but the rebuild — the ledger remains the only history.
///
/// The encoding a snapshot holds (sprawling-SPEC 8-91) leaves out the
/// four fields that are not folded from the ledger; `Views::decode`
/// takes them from `Views::new`.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Views {
    #[serde(skip)]
    pub(super) city_root: PathBuf,
    pub(super) hot: storage::HotView,
    pub(super) attribution: storage::Attribution,
    /// Who may answer, what is waiting, what has already been allowed,
    /// and which scopes a person has shut.
    ///
    /// The reading side of the one governance fold. The worker holds
    /// the judging side, of the same type and folded by the same
    /// `absorb`, so the two sides cannot spell the four arms differently
    /// or disagree about what an unreadable ruling means.
    pub(super) governance: super::Governance,
    pub(super) book: gateway::EndpointBook,
    /// The city's own name, as its first record states it. Handed to a
    /// client at the handshake: the event stream only carries what
    /// happens next, and a browser opened today would otherwise have no
    /// way to learn the name of a city initialised last month.
    pub(super) city: Option<Address>,
    /// The seq of the last record shown to [`Views::apply`], which is
    /// where the served ledger head starts before the fold moves it.
    pub(super) head: Option<kernel::Seq>,
    /// The chain hash of the ledger's first line, which names this
    /// history for its whole life (wire-SPEC, `Welcome.epoch`).
    pub(super) epoch: Option<kernel::B3Hash>,
    /// What waits in each room, folded from the signal records. Held
    /// here rather than read off a queue: a queue answers by being
    /// consumed, and a view that consumed what it showed would change
    /// the thing it reports on.
    pub(super) waiting: std::collections::BTreeMap<Address, Vec<wire::SignalLine>>,
    /// Discarded files, keyed by path so a restoration closes the row
    /// it opened rather than adding a second one.
    pub(super) discards: std::collections::BTreeMap<String, wire::DiscardLine>,
    /// What the city archived, newest last.
    pub(super) assets: Vec<wire::RegistryLine>,
    /// Which run wrote each commit this city made, keyed by the oid the
    /// record announcing it named. Held here rather than read out of
    /// git: the trailers on the commit are a projection of these same
    /// records, and a projection must not be answered from another one.
    pub(super) commits: std::collections::BTreeMap<kernel::GitOid, super::commits::CommitFacts>,
    /// The same commits in the order the history announced them, so a
    /// page can list them newest first without walking the ledger.
    pub(super) commit_seqs: std::collections::BTreeMap<kernel::Seq, kernel::GitOid>,
    /// Which run each successor replaced, folded from `run_started`. A
    /// lineage is walked from here rather than stored per commit, so
    /// the chain is one fact however many commits point into it.
    pub(super) predecessors: std::collections::BTreeMap<kernel::RunId, kernel::RunId>,
    /// The commit each run announced last, so the next one it announces
    /// names its previous commit without a walk past every other run's
    /// (sprawling-SPEC 8-128).
    pub(super) last_commit: std::collections::BTreeMap<kernel::RunId, wire::CommitAt>,
    /// Which runs were frozen with each skill pinned, by name and hash
    /// together: a skill edited between two runs is two documents under
    /// one name, and a key of the name alone would claim the older run
    /// read what the newer one did.
    pub(super) skill_pins: std::collections::BTreeMap<(String, kernel::B3Hash), Vec<kernel::RunId>>,
    /// How many records this view has folded. The one number a page
    /// cannot derive from any other answer.
    pub(super) events: u64,
    /// The first seq not yet folded, which dates every answer read from
    /// here: the answer reflects every record before it. `Seq::FIRST`
    /// until genesis is folded, so "nothing folded" never reads as
    /// "genesis folded".
    pub(super) next_unfolded: kernel::Seq,
    #[serde(
        serialize_with = "super::snapshot::encode_index",
        deserialize_with = "super::snapshot::decode_index"
    )]
    /// seq to byte offset, held rather than rebuilt, and carried by the
    /// views snapshot so a start from it does not rebuild it either
    /// (storage-SPEC 8-4): rebuilt, it cost every history question 14.4
    /// ms on a fifty thousand record ledger.
    ///
    /// Behind a lock of its own because the fold never touches it: a
    /// query carries the `Arc` out of its snapshot of the views and
    /// reads the ledger with only readers waiting on it (sprawling-SPEC.md 8-100).
    pub(super) index: std::sync::Arc<std::sync::Mutex<storage::LedgerIndex>>,
    /// Where each run's first `prompt_assembled` record sits, so the
    /// prompt a page asks for is one ledger line rather than a walk
    /// back through the whole run.
    pub(super) first_prompts: std::collections::BTreeMap<kernel::RunId, kernel::Seq>,
    /// Every building's plan, parsed once and re-parsed only when a
    /// record says it may have moved.
    ///
    /// Behind a lock of its own so a reader reads a plan off the disk
    /// with the views released and puts it back afterwards; the fold
    /// holds it only to forget what a record may have moved
    /// (sprawling-SPEC.md 8-100).
    #[serde(
        serialize_with = "super::snapshot::encode_plans",
        deserialize_with = "super::snapshot::decode_plans"
    )]
    pub(super) plans: std::sync::Arc<std::sync::Mutex<crate::plan_view::PlanView>>,
    /// What each building is working towards, folded from the records
    /// that said so. The goal text and its state, not the value itself:
    /// declaring a pursuit takes the depth-zero position, and a view
    /// that could mint one would be a second door onto the guard.
    pub(super) pursuits: std::collections::BTreeMap<Address, (String, kernel::PursuitState)>,
    /// Every approval this city has answered, oldest first. Appended
    /// rather than keyed, because an answer is a thing that happened
    /// once and the order is what makes the list readable.
    pub(super) decided: Vec<wire::Decision>,
    /// Which runs have held each plan node, folded from
    /// `roadmap_claimed`. It is what turns "what did node 2.3 cost"
    /// into a question `storage::attribution` can answer, and it is a
    /// `BTreeMap` because this is a path a query is answered from.
    pub(super) claims:
        std::collections::BTreeMap<kernel::NodeId, std::collections::BTreeSet<kernel::RunId>>,
    /// What this machine had when the doctor last looked, which is when
    /// somebody last sent `DoctorRefresh`.
    ///
    /// Not folded from anything: this is the one answer here that is
    /// about the machine rather than about the history, which is why it
    /// is set from outside and why a rebuild leaves it alone. `None` is
    /// a city nobody has asked yet - every city that has just been
    /// served - and it answers `Unavailable` rather than an empty
    /// machine.
    #[serde(skip)]
    pub(super) machine: Option<wire::DoctorAnswer>,
    /// The vault the worker opened; set by `views::served`. `None` is a
    /// `Views` nobody served - a rebuild, a test - and a server wanting
    /// a credential then reports that it could not be redeemed rather
    /// than reaching out with the reference as though it were the
    /// value.
    #[serde(skip)]
    pub(super) vault: Option<std::sync::Arc<std::sync::Mutex<gateway::Custodian>>>,
    /// Asks the registry which release is newest; set by
    /// `views::served`. `None` is a `Views` nobody served, and it
    /// answers `Unavailable` rather than leaving this machine.
    #[serde(skip)]
    pub(super) registry: Option<fn() -> wire::ReleaseAnswer>,
    /// How an item's newest release is asked of its publisher; `None`
    /// for views nobody serves, which answer `Unavailable` instead.
    #[serde(skip)]
    pub(super) upstream: Option<fn(&str) -> wire::DoctorUpstream>,
    /// How this machine's search path is asked for one program; set by
    /// `views::served`. `None` is a `Views` nobody served, and the harness
    /// page then answers `Unavailable` rather than reading this machine.
    #[serde(skip)]
    pub(super) programs: Option<fn(&str) -> Option<std::path::PathBuf>>,
    /// The halt a served city's writer refuses lines by until the proof
    /// of its history has a verdict (wire-SPEC.md 8-63); `None` for views
    /// that started from a history proved before they folded it.
    #[serde(skip)]
    pub(super) proof: Option<storage::ChainHalt>,
}

impl Views {
    /// The empty views of the city whose ledger is `ledger_dir`, before a
    /// fold has shown them any record.
    pub fn over(ledger_dir: &Path) -> Views {
        Views::new(city_root_of(ledger_dir))
    }

    pub fn new(city_root: &Path) -> Views {
        // Empty until a fold hands over the index it built, or the first
        // query refreshes it: the history is not read here.
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
            predecessors: std::collections::BTreeMap::new(),
            last_commit: std::collections::BTreeMap::new(),
            skill_pins: std::collections::BTreeMap::new(),
            events: 0,
            next_unfolded: kernel::Seq::FIRST,
            index,
            first_prompts: std::collections::BTreeMap::new(),
            plans,
            pursuits: std::collections::BTreeMap::new(),
            decided: Vec::new(),
            claims: std::collections::BTreeMap::new(),
            machine: None,
            vault: None,
            registry: None,
            upstream: None,
            programs: None,
            proof: None,
        }
    }

    /// Takes the index the fold that built these views read the history
    /// into, so serving does not scan it again, and the epoch its genesis
    /// line's chain hash names, so a page reconnecting to another ledger
    /// rebuilds. Written into the lock this copy shares with its twin, so
    /// both read it.
    ///
    /// # Errors
    /// Propagates a segment the index names that cannot be read.
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
        // The whole index is replaced, so whatever a panic left half
        // written under a poisoned lock is gone with it.
        *self
            .index
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = index;
        self.index.clear_poison();
        Ok(())
    }

    /// The first seq this view has not folded: an answer read from here
    /// reflects every record before it.
    pub(crate) fn next_unfolded(&self) -> kernel::Seq {
        self.next_unfolded
    }

    /// Folds one record into every view that cares about it.
    ///
    /// # Errors
    /// Propagates a view's own refusal to fold a malformed record.
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "a few kinds change what a room holds; the rest of the event vocabulary does not"
    )]
    pub fn apply(&mut self, record: &EventRecord) -> Result<(), AxError> {
        // Before the first change, so an overflow leaves nothing half
        // folded.
        let next_unfolded = record.seq().next()?;
        self.head = Some(record.seq());
        self.hot
            .apply(record)
            .map_err(storage::StorageError::into_ax)?;
        self.attribution
            .apply(record)
            .map_err(storage::StorageError::into_ax)?;
        // A freeze may evict a run and a late record may land on one;
        // either way its money is folded back from the Ledger on demand
        // (sprawling-SPEC section 8-106), so the row goes with it.
        if record.kind() == EventKind::RunFrozen || self.hot.was_evicted(&record.run()) {
            let hot = &self.hot;
            self.attribution.retain_runs(|run| !hot.was_evicted(run));
        }
        self.book.apply(record)?;
        // The one governance fold, shown this line exactly as the
        // worker's own copy is shown it.
        self.governance
            .absorb(record.kind(), record.run(), record.addr(), record.data())?;
        crate::plan_view::PlanView::take_back(&self.plans).apply(record);
        self.events = self.events.saturating_add(1);
        self.next_unfolded = next_unfolded;
        match record.kind() {
            EventKind::CityInitialized => {
                self.city = record.addr().cloned();
            }
            EventKind::SignalEnqueued => {
                let (room, line) = signal_line(record)?;
                self.waiting.entry(room).or_default().push(line);
            }
            EventKind::SignalConsumed => {
                let taken = record
                    .data()
                    .read::<kernel::event::record::SignalConsumed>()?;
                for queue in self.waiting.values_mut() {
                    queue.retain(|held| held.id != taken.id.as_str());
                }
            }
            EventKind::PursuitChanged => {
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
            EventKind::FileDiscarded => {
                for line in discard_lines(record) {
                    self.discards.insert(line.path.clone(), line);
                }
            }
            EventKind::DiscardRestored => {
                for path in restored_paths(record) {
                    if let Some(held) = self.discards.get_mut(&path) {
                        held.restored = true;
                    }
                }
            }
            EventKind::RoadmapClaimed => {
                // The claim names the node; the record names the run
                // that made it. Nothing is removed when the node is put
                // down: what a node cost is what it cost, and a run that
                // released it still spent the money.
                let node = record
                    .data()
                    .read::<kernel::event::record::RoadmapMoved>()?
                    .node;
                self.claims.entry(node).or_default().insert(record.run());
            }
            EventKind::CheckpointCommitted | EventKind::PrMerged => self.fold_commit(record),
            EventKind::RunStarted => {
                self.fold_predecessor(record);
                self.fold_skill_pins(record);
            }
            EventKind::AssetArchived => {
                if let Some(line) = registry_line(record) {
                    self.assets.push(line);
                }
            }
            EventKind::ApprovalResolved => self.fold_ruling(record)?,
            EventKind::PromptAssembled => {
                self.first_prompts
                    .entry(record.run())
                    .or_insert(record.seq());
            }
            _ => {}
        }
        Ok(())
    }

    pub fn epoch(&self) -> Option<kernel::B3Hash> {
        self.epoch
    }

    pub fn head(&self) -> Option<kernel::Seq> {
        self.head
    }

    /// What this city is called: what its first record says, and for a
    /// city made before that record carried a name, the directory it
    /// lives in. One place decides, so two readers cannot disagree.
    pub fn city(&self) -> Option<Address> {
        self.city
            .clone()
            .or_else(|| kernel::layout::CityLayout::new(&self.city_root).city_address())
    }
}
