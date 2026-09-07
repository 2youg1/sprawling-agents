// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a page reads from what the client believes.

use channels::{Address, ApprovalItem, EventRecord, RunId, Seq, UsdMicros};

use crate::app::rows::{ProviderHealth, RunRow, Usage};
use crate::app::snapshot::{Backfill, Snapshot};
use std::collections::BTreeMap;

impl Snapshot {
    /// The four items the right-hand status keeps on screen at all times.
    /// The other eight `status` fields fold away, because twelve permanent
    /// readouts is a tax on attention.
    #[must_use]
    pub fn city(&self) -> Option<&Address> {
        self.city.as_ref()
    }

    #[must_use]
    pub fn spent(&self) -> UsdMicros {
        self.spent
    }

    #[must_use]
    pub fn approvals_pending(&self) -> u32 {
        u32::try_from(self.approvals.len()).unwrap_or(u32::MAX)
    }

    /// What is waiting, oldest arrival first once the inbox groups it.
    /// How many things cannot move until this person answers.
    ///
    /// What the nav badge shows and what the waiting page lists, from one
    /// producer: a badge counting a set the page does not show is worse
    /// than no badge, because a person who clicks it and finds nothing
    /// stops believing the next one.
    ///
    /// Records this client could not read as items are counted in. One
    /// that is not shown is still one that waits, and a badge quietly
    /// short by one is wrong about the only fact it carries.
    #[must_use]
    pub fn waiting_on_you(&self) -> u32 {
        self.approvals_pending()
            .saturating_add(self.unreadable_approvals)
    }

    /// The room a run is in, when this client has folded its start.
    ///
    /// The one answer to "which session is this run", which is what a
    /// link written as `#/live/<uuid>` needs before it can be shown as
    /// the page a person can name.
    #[must_use]
    pub fn room_of(&self, id: &RunId) -> Option<&Address> {
        self.runs.get(id)?.addr.as_ref()
    }

    /// The newest run in a room, and its id.
    ///
    /// Newest rather than every one, because a room is a work line: a
    /// person who opens `lab/parser` means the current state of that
    /// work, and its earlier runs are its history rather than its
    /// siblings. Ordered by the sequence the run started at, so a replay
    /// and a live fold pick the same one.
    #[must_use]
    pub fn session_at(&self, addr: &Address) -> Option<(RunId, &RunRow)> {
        self.runs
            .iter()
            .filter(|(_, row)| row.addr.as_ref() == Some(addr))
            .max_by_key(|(_, row)| row.started_at_seq)
            .map(|(id, row)| (*id, row))
    }

    /// The three counts the top bar keeps on every page: what is moving,
    /// what is stopped on a person, and how many buildings there are.
    ///
    /// Buildings are counted from the rooms this client has seen work
    /// in, so the number is "buildings with work" rather than "folders on
    /// disk" — the city answer holds the second one and says so.
    #[must_use]
    pub fn counts(&self) -> (u32, u32, u32) {
        let mut running = 0u32;
        let mut waiting = 0u32;
        let mut buildings = std::collections::BTreeSet::new();
        for row in self.runs.values() {
            if row.phase.needs_a_person() {
                waiting = waiting.saturating_add(1);
            } else if row.phase.in_flight() {
                running = running.saturating_add(1);
            }
            if let Some(addr) = row.addr.as_ref()
                && let Some((building, _)) = addr.as_str().split_once('/')
            {
                buildings.insert(building.to_owned());
            }
        }
        (
            running,
            waiting,
            u32::try_from(buildings.len()).unwrap_or(u32::MAX),
        )
    }

    /// The sequence this client has folded through, for the line that
    /// says where an answer came from.
    #[must_use]
    pub fn applied_through(&self) -> Option<Seq> {
        self.applied_through
    }

    /// Adds text a model is still saying to the run's display buffer.
    ///
    /// Not `apply`: an increment is not an event, so it does not move
    /// `applied_through`, does not count as having gone live, and cannot
    /// be replayed. A buffer for a run this client never saw start is
    /// kept anyway — the increments arrived, so the run exists, and
    /// discarding them would blank the page a person is watching.
    pub fn is_saying(&mut self, delta: &channels::Delta) {
        self.saying
            .entry(delta.run)
            .or_default()
            .push_str(&delta.text);
    }

    /// What a model is saying in this run, while it is still saying it.
    ///
    /// `None` once the call has settled: the page then draws the record,
    /// which is the text a replay would produce.
    #[must_use]
    pub fn saying(&self, run: &RunId) -> Option<&str> {
        self.saying.get(run).map(String::as_str)
    }

    #[must_use]
    pub fn approvals(&self) -> Vec<ApprovalItem> {
        self.approvals.values().cloned().collect()
    }

    /// Names the city this client is connected to.
    ///
    /// Told by the handshake rather than folded from `city_initialized`:
    /// that record was written when the city was made, and a browser
    /// opened afterwards never sees it. The two agree, because the server
    /// reads the same record to fill the welcome.
    pub fn adopt_city(&mut self, city: Address) {
        self.city = Some(city);
    }

    /// Folds a slice of history this client was not connected for.
    ///
    /// Refused once anything live has been folded. The fold is forward
    /// only - `run_started` after `run_frozen` puts a finished run back
    /// on screen as running - so replaying older records over newer ones
    /// would move the page backwards. Saying so is better than guessing:
    /// a page that has already folded live events is not empty, which is
    /// the condition this exists to fix.
    pub fn backfill(&mut self, records: &[EventRecord]) -> Backfill {
        if self.applied_through.is_some() {
            return Backfill::AlreadyLive;
        }
        let mut folded: usize = 0;
        for record in records {
            if self.apply(record) {
                folded = folded.saturating_add(1);
            }
        }
        Backfill::Folded(folded)
    }

    /// Replaces the pending set with what the server says is pending.
    ///
    /// The stream only carries what happened after this client connected,
    /// so an item raised before that would never appear. The answer is
    /// the authority on the set; the stream advances it from there.
    pub fn adopt_approvals(&mut self, items: Vec<ApprovalItem>) {
        self.approvals = items
            .into_iter()
            .map(|item| (item.id.as_str().to_owned(), item))
            .collect();
    }

    #[must_use]
    pub fn usage(&self) -> Usage {
        self.usage
    }

    #[must_use]
    pub fn unreadable_approvals(&self) -> u32 {
        self.unreadable_approvals
    }

    #[must_use]
    pub fn provider(&self) -> ProviderHealth {
        self.provider
    }

    /// Where a person must go to finish the login this session began.
    #[must_use]
    pub fn login_url(&self) -> Option<&str> {
        self.login_url.as_deref()
    }

    /// What each probed base URL serves. The settings page ticks from
    /// this list; an empty map is a city where nobody has asked yet.
    #[must_use]
    pub fn served(&self) -> &BTreeMap<String, Vec<String>> {
        &self.served
    }

    #[must_use]
    pub fn is_halted(&self) -> bool {
        self.halted
    }

    /// How many signal events this client has folded. Not a queue length -
    /// see the field.
    #[must_use]
    pub fn signals_seen(&self) -> u64 {
        self.signals_seen
    }

    /// Where the stream should resume after a reconnect: one past the last
    /// sequence folded in.
    #[must_use]
    pub fn resume_from(&self) -> Option<Seq> {
        self.applied_through
    }

    pub fn runs(&self) -> impl Iterator<Item = (&RunId, &RunRow)> {
        self.runs.iter()
    }

    #[must_use]
    pub fn run(&self, id: &RunId) -> Option<&RunRow> {
        self.runs.get(id)
    }
}
