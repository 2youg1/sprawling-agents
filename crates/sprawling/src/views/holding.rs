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

use kernel::Address;

// Where a city keeps its ledger and how a building reads off disk are
// `bin::assembly`'s: it forms the city that laid them out. Borrowed
// rather than copied, so "where the ledger lives" keeps one answer.
use super::lines::{buildings_of, verdict_line};
use crate::assembly::{city_address, ledger_dir};

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
            events: 0,
            // An unreadable ledger directory is not a reason to refuse to
            // start: the index is disposable, every refresh tries again,
            // and a city with no ledger yet is the ordinary first run.
            index: memory::LedgerIndex::load_or_rebuild(&ledger_dir(city_root))
                .unwrap_or_else(|_| memory::LedgerIndex::empty()),
            plans: crate::plan_view::PlanView::default(),
            pursuits: std::collections::BTreeMap::new(),
        }
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
