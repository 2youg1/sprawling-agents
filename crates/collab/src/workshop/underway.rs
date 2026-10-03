// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A graph while it is being worked: which of its nodes have already
//! been handed down.
//!
//! A node between ready and joined is in flight, and `done` alone
//! cannot tell it from a node nobody handed down yet. The handed set is
//! what keeps one node from being handed down twice - once when the
//! graph is laid out and again when it is laid out a second time, or
//! when two handbacks land one after another.

use std::collections::BTreeSet;

use kernel::{AxCode, AxError, DelegateKind};

use super::{NodeId, Workshop};
use crate::delegate_tool::{DelegateDesk, Delegated};

/// One room's graph and the nodes it has handed down.
#[derive(Debug)]
pub struct Underway {
    workshop: Workshop,
    handed: BTreeSet<NodeId>,
    /// A desk standing where the run that laid the graph out stood, so a
    /// node handed down after that run froze passes the same two doors
    /// - depth and building - that a node handed down during it passed.
    desk: DelegateDesk,
}

impl Underway {
    /// `handed` is what this room had already handed down before this
    /// graph was laid out; those nodes are not handed down again.
    #[must_use]
    pub fn new(workshop: Workshop, handed: BTreeSet<NodeId>, desk: DelegateDesk) -> Underway {
        Underway {
            workshop,
            handed,
            desk,
        }
    }

    /// Hands down every node that `done` makes ready and that has not
    /// been handed down yet, in id order, and counts them as handed.
    ///
    /// All or nothing: a refusal hands out nothing and counts nothing as
    /// handed, because handed means handed out (`spec/Workshop.lean`), and
    /// a node counted while its request stays in the desk is one no later
    /// call hands down.
    ///
    /// # Errors
    /// Propagates the delegate desk's refusal - the depth and building
    /// rules - and names a scheduled node the graph holds no contract
    /// for.
    pub fn hand_next(&mut self, done: &BTreeSet<NodeId>) -> Result<Vec<Delegated>, AxError> {
        let fresh: Vec<NodeId> = self
            .workshop
            .ready(done)
            .into_iter()
            .filter(|id| !self.handed.contains(id))
            .collect();
        match self.ask_each(&fresh) {
            Ok(()) => {
                self.handed.extend(fresh);
                Ok(self.desk.take())
            }
            Err(refusal) => {
                drop(self.desk.take());
                Err(refusal)
            }
        }
    }

    fn ask_each(&mut self, fresh: &[NodeId]) -> Result<(), AxError> {
        for id in fresh {
            let contract = self.workshop.contract(id).ok_or_else(|| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "hand down a workshop node",
                    format!("{} is scheduled and has no contract", id.as_str()),
                )
                .with_recovery(format!(
                    "remove `{}` from the `depends_on` of every node, or add a node \
                     for it: the schedule holds a room no node describes",
                    id.as_str()
                ))
            })?;
            self.desk.ask(Delegated {
                room: contract.write_domain().clone(),
                task: contract.job_text(),
                goal: contract.done_check().to_owned(),
                kind: DelegateKind::Ephemeral,
            })?;
        }
        Ok(())
    }

    /// Every node handed down so far, this graph's and the room's before
    /// it.
    #[must_use]
    pub fn handed(&self) -> &BTreeSet<NodeId> {
        &self.handed
    }

    /// The order the graph runs in.
    #[must_use]
    pub fn schedule(&self) -> Vec<NodeId> {
        self.workshop.schedule()
    }

    /// Whether every node of the graph has joined, which is when the
    /// room has nothing left to hand down from it.
    #[must_use]
    pub fn is_joined(&self, done: &BTreeSet<NodeId>) -> bool {
        self.workshop.schedule().iter().all(|id| done.contains(id))
    }
}
