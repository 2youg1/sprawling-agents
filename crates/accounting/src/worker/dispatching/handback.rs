// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the run that asked for the work is told when the work comes
//! home.
//!
//! One phase of a dispatch and the last one that speaks to somebody
//! else, which is why it reads on its own rather than at the end of the
//! file that prepares a dispatch and lands it: the reader's question
//! here is what a parent learns, not what a child did.
//!
//! What a graph hands down when a node lands, and when it stops, is
//! specified by `crates/collab/spec/Workshop.lean`; the parent's ending
//! closes the graph in `crate::worker::waking::handing`.

use std::collections::BTreeSet;

use kernel::event::record::SignalId;
use kernel::{Address, AxError, EventKind, Locator, RunId};

use crate::effect;

use super::super::waking::landing::Arrival;
use super::super::{Assignment, CITY_VERIFIER, KnockChain, Owing, RunWorker};
use super::Dispatched;

/// Work the hand-over could not start, and the run that asked for it
/// (accounting D54).
pub(in crate::worker) struct Unstarted<'a> {
    /// The room that asked, where the handback goes.
    pub(in crate::worker) parent: &'a Address,
    /// The run that asked. The child never had a run, so its handback is
    /// written under this one.
    pub(in crate::worker) asker: RunId,
    /// Which request on the asker's delegate desk this was, counted from
    /// zero: two requests for one room are two handbacks.
    pub(in crate::worker) place: usize,
    pub(in crate::worker) child: &'a Address,
    pub(in crate::worker) refusal: &'a AxError,
}

impl RunWorker {
    /// Tells the room that asked for the work that its child could not
    /// start, through the door a child that ran comes back by: the child's
    /// account pinned in the store, a stopped handback carrying the
    /// refusal recorded and then put in the room, a knock for a parent
    /// room nobody is working in, and where the letter landed (collab D19,
    /// accounting D54).
    ///
    /// The letter carries no sender state (collab D10): the child never
    /// had one.
    ///
    /// # Errors
    /// Propagates the store's refusal of the account, a room address that
    /// does not name a node, the ledger's refusal of the lines, the room's
    /// refusal of the letter, and a resident description that cannot be
    /// read.
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

    /// Tells the run that asked for the work how it came back.
    ///
    /// The child's account is pinned in the store before it is judged,
    /// so the locator the parent is handed names bytes rather than a
    /// sentence this process happened to build. The account's one origin
    /// is the child's run and room, so the parent reads it only through
    /// a read bound that covers the child's room (`crates/runtime/Spec.lean`
    /// §8-29-5), and is refused it otherwise. The city verifies:
    /// `Completion::Done` is something the city observed, and a producer
    /// verifying itself is what `Claim::verified` refuses.
    ///
    /// Returns the letter as it arrived in the parent's room: a knock at
    /// that room is made from it, and it owes its `signal_landed` line
    /// once the caller has decided the knock (kernel D38).
    ///
    /// # Errors
    /// Propagates the store's refusal of the account, an address that
    /// does not name a node, and the ledger's refusal of the signal.
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
        // Recorded, then delivered - the same order every other signal
        // takes, so the queue only ever changes as a consequence of a
        // line the history already has.
        self.record_for(
            child.run,
            effect::Line {
                who: child.who.to_owned(),
                addr: parent.clone(),
                kind: EventKind::SignalEnqueued,
                data: signal.enqueued_payload()?,
            },
        )?;
        // Through the room table rather than into a queue of its own:
        // the parent room may have another run reading in it, and a
        // handback delivered beside that reader is one nobody collects.
        // The child has landed: the parent reads how it ended beside the
        // child's address (collab D10).
        let signal = signal.delivered(match child.completion {
            kernel::Completion::Done(_) | kernel::Completion::Limit => collab::SenderState::Frozen,
            kernel::Completion::Cancelled => collab::SenderState::Cancelled,
        });
        let arrival = self.arrive(signal)?;
        // And into the room's join, by the same reading a restart would
        // do: `Handback::from_signal` is the one inverse of the writer
        // just above, so a live delivery and a rebuild cannot disagree
        // about what a handback signal means.
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

    /// Hands down the nodes of the parent room's graph that its join has
    /// just made ready, the way the node that handed back was handed
    /// down: under the same parent run, under the same run policy, owing the same
    /// room. The graph is dropped once every node has joined; a graph its
    /// parent's ending closed is already gone, so nothing more is handed.
    ///
    /// # Errors
    /// Propagates the delegate desk's refusal and whatever starting a
    /// node reports.
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
                    model: None,
                    policy: sibling.policy,
                    origin: None,
                    parent: sibling.parent,
                    succession: None,
                    taint: sibling.taint.clone(),
                    // The same resident handed down both halves of one
                    // workshop, so the sibling's sender is this one's.
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
