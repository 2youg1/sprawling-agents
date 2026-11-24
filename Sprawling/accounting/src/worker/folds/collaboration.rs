// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::PursuitChanged;
use kernel::{Address, AxError, EventKind, EventRecord};

use crate::views::pursued;

use super::super::PlanHolders;

pub(in crate::worker) struct Collaboration {
    pub(in crate::worker) inboxes: std::collections::BTreeMap<Address, collab::Inbox>,

    pub(in crate::worker) joins: std::collections::BTreeMap<Address, collab::FanIn>,
    pub(in crate::worker) goals: Vec<kernel::GoalEntry>,
    pub(in crate::worker) requests: Vec<collab::OpenRequest>,

    pursuits: std::collections::BTreeMap<Address, (String, kernel::PursuitState)>,

    pub(in crate::worker) plan_holders: PlanHolders,
}

impl Collaboration {
    pub(in crate::worker) fn pursuits(
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

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub(super) struct CollaborationFold {
    pub(super) goals: Vec<kernel::GoalEntry>,

    pursuits: std::collections::BTreeMap<Address, (String, kernel::PursuitState)>,

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
                let taken = record
                    .data()
                    .read::<kernel::event::record::SignalConsumed>()?;
                self.consumed.insert(taken.id.as_str().to_owned());
            }
            EventKind::GoalRegistered => super::super::collaborating::register_goal(
                &mut self.goals,
                record.kind(),
                record.data(),
            )?,
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
            EventKind::PrOpened => {
                collab::OpenRequest::from_payload(record.data())?.file(&mut self.requests)
            }

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

pub(in crate::worker) fn new_inbox() -> collab::Inbox {
    collab::Inbox::new(INBOX_CAPACITY, SIGNAL_BANDWIDTH)
}

pub(in crate::worker) const INBOX_CAPACITY: u64 = 256;

pub(super) const SIGNAL_BANDWIDTH: u32 = 4;

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
