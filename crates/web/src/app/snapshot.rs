// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the client believes, folded forward from events; holds no business state.

use std::collections::BTreeMap;

use channels::{Address, ApprovalItem, EventRecord, RunId, Seq, UsdMicros};

use super::rows::{ProviderHealth, RunRow, Usage};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub(super) city: Option<Address>,
    pub(super) runs: BTreeMap<RunId, RunRow>,
    /// The items themselves, keyed by id: the inbox groups by cluster key
    /// and leads with the longest wait, and neither is answerable from a
    /// count. Folded from the stream; the queue query fills in what was
    /// already waiting when this client connected.
    pub(super) approvals: BTreeMap<String, ApprovalItem>,
    /// Approval records this client could not read as items. Counted and
    /// shown: a page that quietly showed one fewer thing waiting would be
    /// wrong about the one fact a person is here to act on.
    pub(super) unreadable_approvals: u32,
    pub(super) spent: UsdMicros,
    pub(super) usage: Usage,
    pub(super) provider: ProviderHealth,
    /// The authorization URL of a subscription login this session began,
    /// held until the login finishes. It lives in the snapshot rather
    /// than in the page because the fact arrives as an event, and every
    /// other fact that arrives as an event is folded here.
    pub(super) login_url: Option<String>,
    /// What each base URL answered when somebody asked what it serves,
    /// keyed by that URL. Held here rather than in the settings page for
    /// the reason `login_url` is: it arrives as an event, and every fact
    /// that arrives as an event is folded in one place.
    pub(super) served: BTreeMap<String, Vec<String>>,
    pub(super) halted: bool,
    /// What a model is saying right now, per run, before the call it
    /// belongs to has settled.
    ///
    /// **Discardable, and discarded.** It is not folded from the ledger,
    /// it does not survive a reload, and `model_returned` throws the
    /// run's buffer away and lets the record speak. Where an increment
    /// and the settled text disagree — a provider that revised, a stream
    /// that was cut — the record wins, because the record is the one a
    /// replay produces.
    pub(super) saying: BTreeMap<RunId, String>,
    /// How many signal events have gone by. A count, never the queue: a
    /// page that folded the queue here would be a second answer to what
    /// waits in a room, and the room's queue is the city's to state. What
    /// this is for is knowing that the answer on screen may have moved.
    pub(super) signals_seen: u64,
    pub(super) applied_through: Option<Seq>,
}

impl Snapshot {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// Rebuilds a snapshot from a stream. The property that matters is stated
/// as a function so tests and a future "reload from scratch" path share one
/// implementation rather than two that can disagree.
#[must_use]
pub fn rebuild<'a>(events: impl IntoIterator<Item = &'a EventRecord>) -> Snapshot {
    let mut snapshot = Snapshot::new();
    for event in events {
        snapshot.apply(event);
    }
    snapshot
}

/// What became of a backfill. Exhaustive, because "nothing was folded"
/// and "this page was already live" are different facts and only one of
/// them is worth saying anything about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backfill {
    Folded(usize),
    AlreadyLive,
}
