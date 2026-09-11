// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The fold every query is answered from, and the lines a page reads off
//! it.
//!
//! **Why it is a projection and not part of the assembly point.** Nothing
//! here decides anything or reaches a provider: it folds records into the
//! answers a client asks for, and `rebuild_views` throws the whole thing
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

use kernel::{Address, AxCode, AxError, EventKind, EventRecord};

// Where a city keeps its ledger and how a building reads off disk are
// `bin::assembly`'s: it forms the city that laid them out. Borrowed
// rather than copied, so "where the ledger lives" keeps one answer.
use super::lines::verdict_line;
use super::lines::{buildings_of, discard_lines, pursuit_from, registry_line, signal_line};
use crate::assembly::{city_address, ledger_dir, rebuild_views};

/// Answers one query out of a city's own history, without serving it.
///
/// The views are folded, asked, and thrown away, so this costs one pass
/// over the ledger and leaves nothing behind. **It is the same
/// [`Views::answer`] a served city answers from**: a command line that
/// read the history its own way would be a second answer to one
/// question, and the one that drifted would be the one nobody was
/// looking at.
///
/// # Errors
/// Propagates a history that does not verify and a record that will not
/// parse. A city whose chain is broken is not one whose views should be
/// handed to anybody.
pub fn ask(city_root: &Path, query: &channels::Query) -> Result<channels::Answer, AxError> {
    Ok(rebuild_views(&ledger_dir(city_root))?.answer(query))
}

/// The derived views a query reads. They are rebuilt from the ledger at
/// startup and folded forward by the write observer, so deleting them
/// costs nothing but the rebuild — the ledger remains the only history.
pub(crate) struct Views {
    pub(super) city_root: PathBuf,
    pub(super) hot: memory::HotView,
    pub(super) attribution: memory::Attribution,
    pub(super) approvals: std::collections::BTreeMap<String, kernel::ApprovalItem>,
    pub(super) book: gateway::EndpointBook,
    /// The city's own name, as its first record states it. Handed to a
    /// client at the handshake: the event stream only carries what
    /// happens next, and a browser opened today would otherwise have no
    /// way to learn the name of a city initialised last month.
    pub(super) city: Option<Address>,
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
    /// How many records this view has folded. The one number a page
    /// cannot derive from any other answer.
    pub(super) events: u64,
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
    /// Who answers for this city, folded from `autonomy_changed`. Held
    /// rather than read off a configuration default: the default is
    /// where a city starts, and what a person changed it to is a line in
    /// the history.
    pub(super) autonomy: kernel::Autonomy,
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
    /// The scopes a halt shut and no release reopened, by the name the
    /// record carries. Folded here as well as in the worker's own
    /// governance, from the same record and the same two words: this is
    /// the reading side, that is the judging side, and a rebuild makes
    /// them equal.
    pub(super) halted: std::collections::BTreeSet<String>,
    /// What this machine had when the city was served, from the one
    /// look the doctor takes at start-up (card-9.2).
    ///
    /// Not folded from anything: this is the one answer here that is
    /// about the machine rather than about the history, which is why it
    /// is set from outside and why a rebuild leaves it alone. `None` is
    /// a city that never looked - a worker driven one command at a time
    /// - and it answers `Unavailable` rather than an empty machine.
    pub(super) machine: Option<channels::DoctorAnswer>,
}

impl Views {
    pub(crate) fn new(city_root: &Path) -> Views {
        Views {
            city_root: city_root.to_path_buf(),
            hot: memory::HotView::new(),
            attribution: memory::Attribution::new(),
            approvals: std::collections::BTreeMap::new(),
            book: gateway::EndpointBook::new(),
            city: None,
            waiting: std::collections::BTreeMap::new(),
            discards: std::collections::BTreeMap::new(),
            assets: Vec::new(),
            commits: std::collections::BTreeMap::new(),
            commit_seqs: std::collections::BTreeMap::new(),
            predecessors: std::collections::BTreeMap::new(),
            events: 0,
            // An unreadable ledger directory is not a reason to refuse to
            // start: the index is disposable, every refresh tries again,
            // and a city with no ledger yet is the ordinary first run.
            index: memory::LedgerIndex::load_or_rebuild(&ledger_dir(city_root))
                .unwrap_or_else(|_| memory::LedgerIndex::empty()),
            plans: crate::plan_view::PlanView::default(),
            pursuits: std::collections::BTreeMap::new(),
            autonomy: kernel::consts_policy::AUTONOMY_DEFAULT,
            decided: Vec::new(),
            claims: std::collections::BTreeMap::new(),
            halted: std::collections::BTreeSet::new(),
            machine: None,
        }
    }

    /// Takes what the doctor found, so a page can be told what this
    /// machine is missing.
    ///
    /// Once, where a city is served. Probing is seconds of starting
    /// programs, and a read that did it would hold the one thread every
    /// other read is answered on.
    pub(crate) fn found_on_this_machine(&mut self, report: channels::DoctorAnswer) {
        self.machine = Some(report);
    }

    /// Folds one record into every view that cares about it.
    ///
    /// # Errors
    /// Propagates a view's own refusal to fold a malformed record.
    pub(crate) fn apply(&mut self, record: &EventRecord) -> Result<(), AxError> {
        self.hot
            .apply(record)
            .map_err(memory::MemoryError::into_ax)?;
        self.attribution
            .apply(record)
            .map_err(memory::MemoryError::into_ax)?;
        self.book.apply(record)?;
        self.plans.apply(record);
        self.events = self.events.saturating_add(1);
        match record.kind() {
            EventKind::CityInitialized => {
                self.city = record.addr().cloned();
            }
            EventKind::SignalEnqueued => {
                if let Some((room, line)) = signal_line(record) {
                    self.waiting.entry(room).or_default().push(line);
                }
            }
            EventKind::SignalConsumed => {
                if let Some((room, line)) = signal_line(record)
                    && let Some(queue) = self.waiting.get_mut(&room)
                {
                    queue.retain(|held| held.id != line.id);
                }
            }
            EventKind::PursuitChanged => {
                if let Some((addr, held)) = pursuit_from(record) {
                    match held {
                        Some(entry) => {
                            self.pursuits.insert(addr, entry);
                        }
                        None => {
                            self.pursuits.remove(&addr);
                        }
                    }
                }
            }
            EventKind::FileDiscarded => {
                for line in discard_lines(record) {
                    self.discards.insert(line.path.clone(), line);
                }
            }
            EventKind::DiscardRestored => {
                for line in discard_lines(record) {
                    if let Some(held) = self.discards.get_mut(&line.path) {
                        held.restored = true;
                    }
                }
            }
            EventKind::RoadmapClaimed => {
                // The claim names the node; the record names the run
                // that made it. Nothing is removed when the node is put
                // down: what a node cost is what it cost, and a run that
                // released it still spent the money.
                if let Some(node) = record
                    .data()
                    .as_map()
                    .get("node")
                    .and_then(serde_json::Value::as_str)
                    .and_then(|held| kernel::NodeId::parse(held).ok())
                {
                    self.claims.entry(node).or_default().insert(record.run());
                }
            }
            EventKind::CheckpointCommitted | EventKind::PrMerged => self.fold_commit(record),
            EventKind::RunStarted => self.fold_predecessor(record),
            EventKind::AssetArchived => {
                if let Some(line) = registry_line(record) {
                    self.assets.push(line);
                }
            }
            EventKind::ApprovalRequested => {
                // The payload *is* the item: it was written by serialising
                // one, so it reads back as one. Rebuilding a lesser shape
                // out of hand-picked fields is how this view came to show
                // every waiting item as "(no summary recorded)" - the field
                // it read had never been written by anybody.
                let value = serde_json::Value::Object(record.data().as_map().clone());
                let item: kernel::ApprovalItem = serde_json::from_value(value).map_err(|err| {
                    AxError::failure(
                        AxCode::WireMismatch,
                        "fold an approval into the queue",
                        format!("seq {}: {err}", record.seq().value()),
                    )
                    .with_recovery(
                        "the record stands; this view skips it and the observer reports it",
                    )
                })?;
                self.approvals.insert(item.id.as_str().to_owned(), item);
            }
            EventKind::ApprovalResolved => {
                let data = record.data().as_map();
                if let Some(id) = data.get("id").and_then(serde_json::Value::as_str) {
                    self.approvals.remove(id);
                    // The cluster travels with the answer because the
                    // person answered the group they were shown; a row
                    // whose cluster will not read back is still an
                    // answer that happened, so it lands with the class
                    // it was recorded under rather than being dropped.
                    let cluster = data
                        .get("cluster")
                        .cloned()
                        .and_then(|value| serde_json::from_value(value).ok())
                        .unwrap_or(kernel::ClusterKey {
                            class: kernel::ApprovalClass::AgentQuestion,
                            detail: String::new(),
                        });
                    let verdict = match data.get("verdict").and_then(serde_json::Value::as_str) {
                        Some("deny") => kernel::PolicyVerdict::Deny,
                        _ => kernel::PolicyVerdict::Allow,
                    };
                    self.decided.push(channels::Decision {
                        item: id.to_owned(),
                        verdict,
                        cluster,
                        at: record.t(),
                    });
                }
            }
            EventKind::CityHalted => {
                let data = record.data().as_map();
                let scope = data.get("scope").and_then(serde_json::Value::as_str);
                let state = data.get("state").and_then(serde_json::Value::as_str);
                match (scope, state) {
                    (Some(scope), Some(crate::assembly::HALTED)) => {
                        self.halted.insert(scope.to_owned());
                    }
                    (Some(scope), Some(crate::assembly::RELEASED)) => {
                        self.halted.remove(scope);
                    }
                    _ => {}
                }
            }
            EventKind::AutonomyChanged => {
                if let Some(name) = record
                    .data()
                    .as_map()
                    .get("autonomy")
                    .and_then(serde_json::Value::as_str)
                {
                    self.autonomy = crate::assembly::read_autonomy(name);
                }
            }
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
                verdict: verdict_line(kernel::observe_pursuit(state, &ready, in_flight)),
                addr,
            });
        }
        out
    }

    /// What this city is called: what its first record says, and for a
    /// city made before that record carried a name, the directory it
    /// lives in. One place decides, so two readers cannot disagree.
    pub(crate) fn city(&self) -> Option<Address> {
        self.city.clone().or_else(|| city_address(&self.city_root))
    }
}
