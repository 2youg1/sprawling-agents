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

use super::lines::verdict_line;
use super::lines::{
    buildings_of, discard_lines, pursued, registry_line, restored_paths, signal_line,
};

/// Answers one query out of a city's own history, without serving it.
///
/// The views are folded, asked, and thrown away, so this costs one pass
/// over the ledger and leaves nothing behind. **It is the same
/// [`Views::prepare`] a served city answers from**: a command line that
/// read the history its own way would be a second answer to one
/// question, and the one that drifted would be the one nobody was
/// looking at.
///
/// # Errors
/// Propagates a history that does not verify and a record that will not
/// parse. A city whose chain is broken is not one whose views should be
/// handed to anybody.
pub fn ask(city_root: &Path, query: &channels::Query) -> Result<channels::Answer, AxError> {
    Ok(
        Views::rebuild(&kernel::layout::CityLayout::new(city_root).ledger())?
            .prepare(query)
            .finish(),
    )
}

/// The derived views a query reads. They are rebuilt from the ledger at
/// startup and folded forward by the write observer, so deleting them
/// costs nothing but the rebuild — the ledger remains the only history.
pub(crate) struct Views {
    pub(super) city_root: PathBuf,
    pub(super) hot: memory::HotView,
    pub(super) attribution: memory::Attribution,
    /// Who may answer, what is waiting, what has already been allowed,
    /// and which scopes a person has shut.
    ///
    /// The reading side of the one governance fold. The worker holds
    /// the judging side, of the same type and folded by the same
    /// `absorb`: this view used to spell the four arms a second time,
    /// and the two spellings disagreed about what an unreadable ruling
    /// meant.
    pub(super) governance: super::Governance,
    pub(super) book: gateway::EndpointBook,
    /// The city's own name, as its first record states it. Handed to a
    /// client at the handshake: the event stream only carries what
    /// happens next, and a browser opened today would otherwise have no
    /// way to learn the name of a city initialised last month.
    pub(super) city: Option<Address>,
    /// The seq of the last record shown to [`Views::apply`], which is
    /// where the served ledger head starts before the fold moves it.
    head: Option<kernel::Seq>,
    /// The chain hash of the ledger's first line, which names this
    /// history for its whole life (channels-SPEC, `Welcome.epoch`).
    epoch: Option<kernel::B3Hash>,
    /// What waits in each room, folded from the signal records. Held
    /// here rather than read off a queue: a queue answers by being
    /// consumed, and a view that consumed what it showed would change
    /// the thing it reports on.
    pub(super) waiting: std::collections::BTreeMap<Address, Vec<channels::SignalLine>>,
    /// Discarded files, keyed by path so a restoration closes the row
    /// it opened rather than adding a second one.
    pub(super) discards: std::collections::BTreeMap<String, channels::DiscardLine>,
    /// What the city archived, newest last.
    pub(super) assets: Vec<channels::RegistryLine>,
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
    /// seq to byte offset, held rather than rebuilt.
    ///
    /// Rebuilding it read the whole side cache and allocated a `String`
    /// per line, and that was charged to every history question a page
    /// asked - 14.4 ms of it on a fifty thousand record ledger. Held, the
    /// same question costs one directory listing and the bytes that are
    /// actually new.
    pub(super) index: memory::LedgerIndex,
    /// Every building's plan, parsed once and re-parsed only when a
    /// record says it may have moved.
    pub(super) plans: crate::plan_view::PlanView,
    /// What each building is working towards, folded from the records
    /// that said so. The goal text and its state, not the value itself:
    /// declaring a pursuit takes the depth-zero position, and a view
    /// that could mint one would be a second door onto the guard.
    pub(super) pursuits: std::collections::BTreeMap<Address, (String, kernel::PursuitState)>,
    /// Every approval this city has answered, oldest first. Appended
    /// rather than keyed, because an answer is a thing that happened
    /// once and the order is what makes the list readable.
    pub(super) decided: Vec<channels::Decision>,
    /// Which runs have held each plan node, folded from
    /// `roadmap_claimed`. It is what turns "what did node 2.3 cost"
    /// into a question `memory::attribution` can answer, and it is a
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
    pub(super) machine: Option<channels::DoctorAnswer>,
    /// The vault the worker opened; set by `views::served`. `None` is a
    /// `Views` nobody served - a rebuild, a test - and a server wanting
    /// a credential then reports that it could not be redeemed rather
    /// than reaching out with the reference as though it were the
    /// value.
    pub(super) vault: Option<std::sync::Arc<std::sync::Mutex<gateway::Custodian>>>,
}

impl Views {
    /// Rebuilds the views from the ledger on disk. This is the
    /// disposability of a projection exercised on every start: nothing
    /// is persisted, and the answer is the same as if the process had
    /// been running all along.
    ///
    /// # Errors
    /// Propagates chain verification failures; a city whose history does
    /// not verify is not one whose views should be served.
    pub(crate) fn rebuild(ledger_dir: &Path) -> Result<Views, AxError> {
        let mut views = Views::over(ledger_dir);
        let index = runtime::replay::fold_ledger_dir(ledger_dir, |record| views.apply(record))?;
        views.hold_index(index, ledger_dir)?;
        Ok(views)
    }

    /// The empty views of the city whose ledger is `ledger_dir`, two
    /// levels up, before a fold has shown them any record.
    pub(crate) fn over(ledger_dir: &Path) -> Views {
        Views::new(ledger_dir.ancestors().nth(2).unwrap_or(ledger_dir))
    }

    pub(crate) fn new(city_root: &Path) -> Views {
        Views {
            city_root: city_root.to_path_buf(),
            hot: memory::HotView::new(),
            attribution: memory::Attribution::new(),
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
            skill_pins: std::collections::BTreeMap::new(),
            events: 0,
            next_unfolded: kernel::Seq::FIRST,
            // Empty until a fold hands over the index it built, or the
            // first query refreshes it: the history is not read here.
            index: memory::LedgerIndex::empty(),
            plans: crate::plan_view::PlanView::default(),
            pursuits: std::collections::BTreeMap::new(),
            decided: Vec::new(),
            claims: std::collections::BTreeMap::new(),
            machine: None,
            vault: None,
        }
    }

    /// Takes the index the fold read the history into, so serving does
    /// not scan it again, and the epoch its genesis line's chain hash
    /// names, so a page reconnecting to another ledger rebuilds.
    ///
    /// # Errors
    /// Propagates a segment the index names that cannot be read.
    pub(crate) fn hold_index(
        &mut self,
        index: memory::LedgerIndex,
        ledger_dir: &Path,
    ) -> Result<(), AxError> {
        self.epoch = match index.reader(ledger_dir).line_at(kernel::Seq::FIRST) {
            Ok(genesis) => Some(kernel::ledger::chain_hash(&genesis)),
            Err(memory::MemoryError::SeqMissing { .. }) => None,
            Err(other) => return Err(other.into_ax()),
        };
        self.index = index;
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
    pub(crate) fn apply(&mut self, record: &EventRecord) -> Result<(), AxError> {
        self.head = Some(record.seq());
        self.hot
            .apply(record)
            .map_err(memory::MemoryError::into_ax)?;
        self.attribution
            .apply(record)
            .map_err(memory::MemoryError::into_ax)?;
        // A freeze may evict a run and a late record may land on one;
        // either way its money is folded back from the Ledger on demand
        // (sprawling-SPEC section 8-90), so the row goes with it.
        if record.kind() == EventKind::RunFrozen || self.hot.was_evicted(&record.run()) {
            let hot = &self.hot;
            self.attribution.retain_runs(|run| !hot.was_evicted(run));
        }
        self.book.apply(record)?;
        // The one governance fold, shown this line exactly as the
        // worker's own copy is shown it.
        self.governance
            .absorb(record.kind(), record.run(), record.addr(), record.data())?;
        self.plans.apply(record);
        self.events = self.events.saturating_add(1);
        self.next_unfolded = record.seq().next()?;
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
                if let Some(queue) = record.addr().and_then(|room| self.waiting.get_mut(room)) {
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
            _ => {}
        }
        Ok(())
    }

    /// One entry per building, with its plan as the projection last read
    /// it.
    pub(super) fn spine(&mut self) -> Vec<channels::BuildingProgress> {
        let root = self.city_root.clone();
        buildings_of(&root)
            .into_iter()
            .map(|addr| {
                let reading = self.plans.of(&root, &addr);
                channels::BuildingProgress {
                    addr,
                    progress: reading.progress,
                    problems: reading.problems,
                    blocked: reading.blocked,
                    ready: u32::try_from(reading.ready.len()).unwrap_or(u32::MAX),
                }
            })
            .collect()
    }

    /// What each pursuit is doing, as the city reads it.
    ///
    /// The verdict is computed here rather than on the page, so the stop
    /// condition has one authority: a client that worked out for itself
    /// whether a city had finished would be the second.
    pub(super) fn pursuit_lines(&mut self) -> Vec<channels::PursuitLine> {
        let root = self.city_root.clone();
        let held: Vec<(Address, String, kernel::PursuitState)> = self
            .pursuits
            .iter()
            .map(|(addr, (goal, state))| (addr.clone(), goal.clone(), *state))
            .collect();
        let in_flight = u32::try_from(self.hot.active_count()).unwrap_or(u32::MAX);
        let mut out = Vec::new();
        for (addr, goal, state) in held {
            let ready = self.plans.of(&root, &addr).ready;
            out.push(channels::PursuitLine {
                goal,
                state,
                verdict: verdict_line(kernel::pursuit::observe(state, &ready, in_flight)),
                addr,
            });
        }
        out
    }

    pub(crate) fn epoch(&self) -> Option<kernel::B3Hash> {
        self.epoch
    }

    pub(crate) fn head(&self) -> Option<kernel::Seq> {
        self.head
    }

    /// What this city is called: what its first record says, and for a
    /// city made before that record carried a name, the directory it
    /// lives in. One place decides, so two readers cannot disagree.
    pub(crate) fn city(&self) -> Option<Address> {
        self.city
            .clone()
            .or_else(|| kernel::layout::CityLayout::new(&self.city_root).city_address())
    }
}
