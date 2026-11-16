// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;

use kernel::event::record::SignalId;
use kernel::{Address, AxError, EventKind, Locator, RunId};

use crate::effect;

use super::super::waking::landing::Arrival;
use super::super::{Assignment, CITY_VERIFIER, KnockChain, Owing, RunWorker};
use super::Dispatched;

pub(in crate::worker) struct Unstarted<'a> {

    pub(in crate::worker) parent: &'a Address,

    pub(in crate::worker) asker: RunId,

    pub(in crate::worker) place: usize,
    pub(in crate::worker) child: &'a Address,
    pub(in crate::worker) refusal: &'a AxError,
}

impl RunWorker {

    pub(in crate::worker) fn deliver_unstarted(
        &mut self,
        unstarted: Unstarted<'_>,
        policy: kernel::RunPolicy,
        chain: &KnockChain,
    ) -> Result<(), AxError> {
        let Unstarted {
            parent,
            asker,
            place,
            child,
            refusal,
        } = unstarted;
        let account = format!(
            "room: {}
ending: not started
refusal: {refusal}
recovery: {}
",
            child.as_str(),
            refusal.recovery()
        );
        let digest = self
            .cas
            .put_for(
                account.as_bytes(),
                &storage::BlockOrigin {
                    run: asker,
                    building: child.clone(),
                },
            )
            .map_err(storage::StorageError::into_ax)?;
        let by = child.as_str();
        let claim = collab::Claim::new(
            collab::NodeId::parse(by)?,
            Locator::cas(digest),
            digest,
            by.to_owned(),
        );
        let signal = collab::Handback::unstarted(claim, refusal).signal(
            SignalId::parse(&format!("handback-{asker}-{place}"))?,
            parent.clone(),
            self.clock.now()?,
        )?;
        self.record_for(
            asker,
            effect::Line {
                who: by.to_owned(),
                addr: parent.clone(),
                kind: EventKind::SignalEnqueued,
                data: signal.enqueued_payload()?,
            },
        )?;
        let arrival = self.arrive(signal)?;
        self.knock(arrival.signal(), child, policy, chain)?;
        self.record_landing(arrival, asker, by)
    }

    pub(in crate::worker) fn deliver_handback(
        &mut self,
        parent: &Address,
        child: &Dispatched,
    ) -> Result<Arrival, kernel::AxError> {
        let account = format!(
            "room: {}\nby: {}\nending: {}\n",
            child.addr.as_str(),
            child.who,
            child.completion.name()
        );
        let digest = self
            .cas
            .put_for(
                account.as_bytes(),
                &storage::BlockOrigin {
                    run: child.run,
                    building: child.addr.clone(),
                },
            )
            .map_err(storage::StorageError::into_ax)?;
        let claim = collab::Claim::new(
            collab::NodeId::parse(child.addr.as_str())?,
            Locator::cas(digest),
            digest,
            child.who.clone(),
        );
        let back = collab::Handback::of(
            claim,
            matches!(child.completion, kernel::Completion::Done(_)),
            CITY_VERIFIER,
        );
        let signal = back.signal(
            kernel::event::record::SignalId::parse(&format!("handback-{}", child.run))?,
            parent.clone(),
            self.clock.now()?,
        )?;

        self.record_for(
            child.run,
            effect::Line {
                who: child.who.to_owned(),
                addr: parent.clone(),
                kind: EventKind::SignalEnqueued,
                data: signal.enqueued_payload()?,
            },
        )?;

        let signal = signal.delivered(match child.completion {
            kernel::Completion::Done(_) | kernel::Completion::Limit => collab::SenderState::Frozen,
            kernel::Completion::Cancelled => collab::SenderState::Cancelled,
        });
        let arrival = self.arrive(signal)?;

        if let Some(collab::Handback::Finished(artifact)) =
            collab::Handback::from_signal(arrival.signal())?
        {
            self.collaborating
                .joins
                .entry(parent.clone())
                .or_default()
                .accept(artifact);
        }
        Ok(arrival)
    }

    pub(in crate::worker) fn hand_down_what_is_ready(
        &mut self,
        parent: &Address,
        sibling: &Assignment,
        owing: &Owing,
    ) -> Result<(), kernel::AxError> {
        let done: BTreeSet<collab::NodeId> =
            self.collaborating
                .joins
                .get(parent)
                .map_or_else(BTreeSet::new, |join| {
                    join.artifacts()
                        .map(|artifact| artifact.node().clone())
                        .collect()
                });
        let Some(underway) = self.collaborating.workshops.get_mut(parent) else {
            return Ok(());
        };
        let ready = underway.hand_next(&done)?;
        if underway.is_joined(&done) {
            self.collaborating.workshops.remove(parent);
        }
        for work in ready {
            self.note(
                runtime::diagnostics::Level::Effect,
                "collab::workshop",
                &format!("{} handed work to {}", parent.as_str(), work.room.as_str()),
            );
            self.dispatch_into_lane(
                Assignment {
                    addr: work.room,
                    session: None,
                    effort: None,
                    difficulty: work.difficulty,
                    model: None,
                    policy: sibling.policy,
                    origin: None,
                    parent: sibling.parent,
                    succession: None,
                    taint: sibling.taint.clone(),

                    dispatched_by: sibling.dispatched_by.clone(),
                },
                work.task,
                work.goal,
                owing.child(parent.clone()),
            )?;
        }
        Ok(())
    }
}
