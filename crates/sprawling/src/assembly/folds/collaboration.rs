// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The two projections the collaboration tools read: what is waiting in
//! each room, and what ground is already claimed.

use kernel::event::record::PursuitChanged;
use kernel::{Address, AxError, EventKind, EventRecord};

use crate::views::pursued;

use super::super::PlanHolders;

/// The three registers a run's collaboration tools read from.
pub(in crate::assembly) struct Collaboration {
    pub(in crate::assembly) inboxes: std::collections::BTreeMap<Address, collab::Inbox>,
    /// What each room's earlier runs got back from work they handed
    /// down, verified. Folded from the same handback signals the inboxes
    /// are folded from, because a join outlives one run: a child starts
    /// after its parent froze.
    pub(in crate::assembly) joins: std::collections::BTreeMap<Address, collab::FanIn>,
    pub(in crate::assembly) goals: Vec<kernel::GoalEntry>,
    pub(in crate::assembly) requests: Vec<collab::OpenRequest>,
    /// What each building was last told to work towards.
    pursuits: std::collections::BTreeMap<Address, (String, kernel::PursuitState)>,
    /// Which room holds each node of each building's plan.
    pub(in crate::assembly) plan_holders: PlanHolders,
}

impl Collaboration {
    /// The pursuits as values, minted through the depth-zero position.
    ///
    /// The fold reads text and state out of the records; only a holder
    /// of a `Delegator` turns those back into something that can take
    /// work, which is why this takes one rather than doing it inline.
    pub(in crate::assembly) fn pursuits(
        &self,
        at: &kernel::Delegator,
    ) -> std::collections::BTreeMap<Address, kernel::Pursuit> {
        let mut out = std::collections::BTreeMap::new();
        for (addr, (goal, state)) in &self.pursuits {
            let Ok(mut held) = kernel::Pursuit::declare(at, goal.clone()) else {
                continue;
            };
            if *state == kernel::PursuitState::Paused {
                held.pause();
            }
            out.insert(addr.clone(), held);
        }
        out
    }
}

/// `Collaboration` while it is still being read out of a history.
///
/// Signals are held aside until the last line has been seen, because a
/// queue is `enqueued` minus `consumed` and the two arrive in whatever
/// order the work happened in. Nothing else here needs a second look, so
/// nothing else is staged.
#[derive(Default, serde::Serialize, serde::Deserialize)]
pub(super) struct CollaborationFold {
    pub(super) goals: Vec<kernel::GoalEntry>,
    /// What each building was last told to work towards. Text and
    /// state, because a `Pursuit` is minted through the depth-zero
    /// position and a fold has none.
    pursuits: std::collections::BTreeMap<Address, (String, kernel::PursuitState)>,
    /// Which room holds each node of each building's plan: the map a
    /// red node's neighbours are found through.
    plan_holders: PlanHolders,
    pub(super) requests: Vec<collab::OpenRequest>,
    #[serde(
        serialize_with = "enqueued_as_text",
        deserialize_with = "enqueued_from_text"
    )]
    enqueued: Vec<collab::Signal>,
    consumed: std::collections::BTreeSet<String>,
}

impl CollaborationFold {
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "a few kinds carry collaboration; the rest of the event vocabulary does not"
    )]
    pub(super) fn absorb(&mut self, record: &EventRecord) -> Result<(), AxError> {
        match record.kind() {
            EventKind::SignalEnqueued => self
                .enqueued
                .push(collab::Signal::from_payload(record.data())?),
            EventKind::SignalConsumed => {
                // Through the writer's own inverse, and refusing what
                // it cannot read: a consumption skipped here is a
                // signal the rebuild believes is still waiting, and a
                // resident is handed it a second time.
                let taken = record
                    .data()
                    .read::<kernel::event::record::SignalConsumed>()?;
                self.consumed.insert(taken.id.as_str().to_owned());
            }
            EventKind::GoalRegistered => self.goals.push(record.data().read()?),
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
            EventKind::PrOpened => self
                .requests
                .push(collab::OpenRequest::from_payload(record.data())?),
            // A request leaves the register whichever way it ended: a
            // merged one is done, and a rejected one goes back to the
            // resident who wrote it rather than sitting in a queue
            // nobody owns.
            EventKind::PrMerged => {
                let branch = record.data().read::<collab::MergedRequest>()?.branch;
                self.requests.retain(|held| held.branch != branch);
            }
            EventKind::PrRejected => {
                let branch = record
                    .data()
                    .read::<collab::RejectedRequest>()?
                    .request
                    .branch;
                self.requests.retain(|held| held.branch != branch);
            }
            // `PlanHolders` alone decides which kinds move a claim, so
            // this fold cannot list one kind fewer than the live table.
            _ => self
                .plan_holders
                .absorb(record.kind(), record.addr(), record.data())?,
        }
        Ok(())
    }

    pub(super) fn settle(self) -> Result<Collaboration, AxError> {
        let mut inboxes = std::collections::BTreeMap::new();
        let mut joins: std::collections::BTreeMap<Address, collab::FanIn> =
            std::collections::BTreeMap::new();
        let (goals, requests, consumed) = (self.goals, self.requests, self.consumed);
        let (pursuits, plan_holders) = (self.pursuits, self.plan_holders);
        for signal in self.enqueued {
            // A join is folded from every handback the room ever
            // received, whether or not the signal announcing it has been
            // read: reading a notice and holding a result are different
            // facts.
            if let Some(collab::Handback::Finished(artifact)) =
                collab::Handback::from_signal(&signal)?
            {
                joins
                    .entry(signal.room().clone())
                    .or_default()
                    .accept(artifact);
            }
            if consumed.contains(signal.id().as_str()) {
                continue;
            }
            let inbox = inboxes
                .entry(signal.room().clone())
                .or_insert_with(new_inbox);
            inbox.deliver(&signal)?;
        }
        Ok(Collaboration {
            inboxes,
            joins,
            goals,
            requests,
            pursuits,
            plan_holders,
        })
    }
}

pub(in crate::assembly) fn new_inbox() -> collab::Inbox {
    collab::Inbox::new(INBOX_CAPACITY, SIGNAL_BANDWIDTH)
}

/// How many signals one room may hold, and how many one pull takes.
/// Bandwidth belongs to the receiver: a sender cannot push more into a
/// resident's context than the resident agreed to read at once.
pub(in crate::assembly) const INBOX_CAPACITY: u64 = 256;

pub(super) const SIGNAL_BANDWIDTH: u32 = 4;

/// The signals still waiting, in a snapshot, as the `signal_enqueued`
/// payloads they were folded from: the writer's own inverse reads them
/// back, and a payload is JSON, which postcard cannot carry.
fn enqueued_as_text<S: serde::Serializer>(
    enqueued: &[collab::Signal],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let payloads = enqueued
        .iter()
        .map(collab::Signal::enqueued_payload)
        .collect::<Result<Vec<_>, AxError>>()
        .map_err(serde::ser::Error::custom)?;
    super::as_json_text(&payloads, serializer)
}

fn enqueued_from_text<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<collab::Signal>, D::Error> {
    super::from_json_text::<Vec<kernel::Payload>, D>(deserializer)?
        .iter()
        .map(collab::Signal::from_payload)
        .collect::<Result<Vec<_>, AxError>>()
        .map_err(serde::de::Error::custom)
}
