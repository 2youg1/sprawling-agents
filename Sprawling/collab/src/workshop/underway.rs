// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;

use kernel::{AxCode, AxError, DelegateKind};

use super::{NodeId, Workshop};
use crate::delegate_tool::{DelegateDesk, Delegated};

#[derive(Debug)]
pub struct Underway {
    workshop: Workshop,
    handed: BTreeSet<NodeId>,

    desk: DelegateDesk,
}

impl Underway {
    #[must_use]
    pub fn new(workshop: Workshop, handed: BTreeSet<NodeId>, desk: DelegateDesk) -> Underway {
        Underway {
            workshop,
            handed,
            desk,
        }
    }

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
                difficulty: None,
            })?;
        }
        Ok(())
    }

    #[must_use]
    pub fn handed(&self) -> &BTreeSet<NodeId> {
        &self.handed
    }

    #[must_use]
    pub fn schedule(&self) -> Vec<NodeId> {
        self.workshop.schedule()
    }

    #[must_use]
    pub fn is_joined(&self, done: &BTreeSet<NodeId>) -> bool {
        self.workshop.schedule().iter().all(|id| done.contains(id))
    }
}

#[derive(Debug)]
pub(crate) enum Laid {
    Nothing,
    Held(Underway),
    Taken,
}

impl Laid {
    pub(crate) fn take(&mut self) -> Option<Underway> {
        match std::mem::replace(self, Laid::Taken) {
            Laid::Held(underway) => Some(underway),
            Laid::Nothing => {
                *self = Laid::Nothing;
                None
            }
            Laid::Taken => None,
        }
    }
}
