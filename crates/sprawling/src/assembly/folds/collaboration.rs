// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The two projections the collaboration tools read: what is waiting in
//! each room, and what ground is already claimed.

use kernel::{Address, AxError, EventKind};
use kernel::{EventRecord, Locator};

use crate::effect;
use crate::views::pursuit_from;

use super::super::{building_of, plan_node_of};

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
    pub(in crate::assembly) plan_holders:
        std::collections::BTreeMap<Address, std::collections::BTreeMap<kernel::NodeId, String>>,
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
#[derive(Default)]
pub(super) struct CollaborationFold {
    pub(super) goals: Vec<kernel::GoalEntry>,
    /// What each building was last told to work towards. Text and
    /// state, because a `Pursuit` is minted through the depth-zero
    /// position and a fold has none.
    pursuits: std::collections::BTreeMap<Address, (String, kernel::PursuitState)>,
    /// Which room holds each node of each building's plan. The map a
    /// red node's neighbours are found through, and the one copy of it:
    /// the record that claims a node carries the room in its `addr`,
    /// so nothing here derives what somebody else already wrote down.
    plan_holders:
        std::collections::BTreeMap<Address, std::collections::BTreeMap<kernel::NodeId, String>>,
    pub(super) requests: Vec<collab::OpenRequest>,
    enqueued: Vec<collab::Signal>,
    consumed: std::collections::BTreeSet<String>,
}

impl CollaborationFold {
    pub(super) fn absorb(&mut self, record: &EventRecord) -> Result<(), AxError> {
        match record.kind() {
            EventKind::SignalEnqueued => self
                .enqueued
                .push(collab::Signal::from_payload(record.data())?),
            EventKind::SignalConsumed => {
                if let Some(id) = record
                    .data()
                    .as_map()
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                {
                    self.consumed.insert(id.to_owned());
                }
            }
            EventKind::GoalRegistered => self.goals.push(effect::goal_from_payload(record.data())?),
            EventKind::RoadmapClaimed => {
                if let (Some(building), Some(node), Some(room)) = (
                    record.addr().and_then(building_of),
                    plan_node_of(record),
                    record.addr(),
                ) {
                    self.plan_holders
                        .entry(building)
                        .or_default()
                        .insert(node, room.as_str().to_owned());
                }
            }
            EventKind::RoadmapFinished | EventKind::RoadmapReleased | EventKind::RoadmapBlocked => {
                if let (Some(building), Some(node)) =
                    (record.addr().and_then(building_of), plan_node_of(record))
                {
                    self.plan_holders.entry(building).or_default().remove(&node);
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
            EventKind::PrOpened => self
                .requests
                .push(collab::OpenRequest::from_payload(record.data())?),
            // A request leaves the register whichever way it ended: a
            // merged one is done, and a rejected one goes back to the
            // resident who wrote it rather than sitting in a queue
            // nobody owns.
            EventKind::PrMerged | EventKind::PrRejected => {
                if let Some(branch) = record
                    .data()
                    .as_map()
                    .get("branch")
                    .and_then(serde_json::Value::as_str)
                {
                    self.requests.retain(|held| held.branch != branch);
                }
            }
            _ => {}
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
            if let Some(artifact) = artifact_of(&signal) {
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

/// The verified result a handback signal reports, when it reports one.
///
/// Reads back what `collab::Handback::signal` wrote, and nothing else -
/// an ordinary signal between residents is not a result and returns
/// `None`. The digest is taken from the locator rather than carried
/// beside it: two fields holding one hash are two places for it to
/// disagree.
pub(in crate::assembly) fn artifact_of(signal: &collab::Signal) -> Option<collab::Artifact> {
    let body = signal.payload().as_map();
    if body.get("handback").and_then(serde_json::Value::as_str)? != "finished" {
        return None;
    }
    let node = collab::NodeId::parse(body.get("room").and_then(serde_json::Value::as_str)?).ok()?;
    let at = Locator::parse(body.get("at").and_then(serde_json::Value::as_str)?).ok()?;
    let Locator::Cas { hash, .. } = at else {
        return None;
    };
    let verified_by = body
        .get("verified_by")
        .and_then(serde_json::Value::as_str)?
        .to_owned();
    collab::Claim::new(node, at.clone(), hash, signal.from().to_owned())
        .verified(true, &verified_by)
        .ok()
}

pub(in crate::assembly) fn new_inbox() -> collab::Inbox {
    collab::Inbox::new(INBOX_CAPACITY, SIGNAL_BANDWIDTH)
}

/// How many signals one room may hold, and how many one pull takes.
/// Bandwidth belongs to the receiver: a sender cannot push more into a
/// resident's context than the resident agreed to read at once.
pub(super) const INBOX_CAPACITY: u64 = 256;

pub(super) const SIGNAL_BANDWIDTH: u32 = 4;
