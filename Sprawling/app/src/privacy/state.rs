// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::num::NonZeroU64;

use kernel::{AxError, SecretRef};
use serde::{Deserialize, Serialize};
use wire::{PrivacyControl, PrivacySettlement};

use super::fault::HistoryFault;
use super::target::Snapshot;

pub(super) const SCHEMA: u32 = 3;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Intent {
    pub(super) operation: NonZeroU64,
    pub(super) control: PrivacyControl,
    pub(super) owner: SecretRef,
    pub(super) original: Snapshot,
    pub(super) modified: Snapshot,

    pub(super) key_existed: bool,
    pub(super) restore_of: Option<NonZeroU64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Outcome {
    Applied,
    NotApplied,
    Restored,
    RolledBack,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "phase", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Event {
    Prepared {
        intent: Intent,
    },
    Finished {
        operation: NonZeroU64,
        outcome: Outcome,
    },
    Reconciled {
        operation: NonZeroU64,
        settlement: PrivacySettlement,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Line {
    pub(super) schema: u32,
    pub(super) event: Event,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum StatusOutcome {
    Unresolved,
    Finished(Outcome),
    Reconciled(PrivacySettlement),
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub(super) struct Status {
    operation: NonZeroU64,
    outcome: StatusOutcome,
}

#[derive(Debug)]
struct Summary {
    owner: SecretRef,
    statuses: Vec<Status>,
}

#[derive(Debug, Default)]
pub(super) struct History(Fold);

#[derive(Debug)]
pub(super) struct Holdings<'h, O = SecretRef> {
    owner: O,
    owned: &'h BTreeMap<PrivacyControl, Vec<Intent>>,
    unresolved: Option<&'h Intent>,
    latest: Option<NonZeroU64>,
}

#[derive(Debug)]
struct Pending {
    intent: Intent,
    stage: Stage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Open,

    Unknown,
}

#[derive(Debug, Default)]
struct Fold {
    summary: Option<Summary>,
    owned: BTreeMap<PrivacyControl, Vec<Intent>>,
    pending: Option<Pending>,
    latest: Option<NonZeroU64>,
}

impl History {
    pub(super) fn fold(lines: Vec<Line>) -> Result<Self, HistoryFault> {
        let mut fold = Fold::default();
        for line in lines {
            if line.schema != SCHEMA {
                return Err(HistoryFault::Invalid("unknown history schema"));
            }
            match line.event {
                Event::Prepared { intent } => fold.prepare(intent)?,
                Event::Finished { operation, outcome } => fold.finish(operation, outcome)?,
                Event::Reconciled {
                    operation,
                    settlement,
                } => fold.reconcile(operation, settlement)?,
            }
        }
        Ok(Self(fold))
    }

    pub(super) fn disclose(
        &self,
        authorize: impl FnOnce(&SecretRef) -> Result<(), AxError>,
    ) -> Result<&[Status], AxError> {
        match &self.0.summary {
            None => Ok(&[]),
            Some(summary) => authorize(&summary.owner).map(|()| summary.statuses.as_slice()),
        }
    }

    pub(super) fn holdings<O>(
        &self,
        authorize: impl FnOnce(Option<&SecretRef>) -> Result<O, AxError>,
    ) -> Result<Holdings<'_, O>, AxError> {
        let owner = authorize(self.0.summary.as_ref().map(|summary| &summary.owner))?;
        Ok(Holdings {
            owner,
            owned: &self.0.owned,
            unresolved: self.0.pending.as_ref().map(|pending| &pending.intent),
            latest: self.0.latest,
        })
    }
}

impl Holdings<'_> {
    pub(super) fn owner(&self) -> &SecretRef {
        &self.owner
    }

    pub(super) fn next_operation(&self) -> Option<NonZeroU64> {
        match self.latest {
            None => Some(NonZeroU64::MIN),
            Some(latest) => latest.checked_add(1),
        }
    }
}

impl<O> Holdings<'_, O> {
    pub(super) fn unresolved(&self) -> Option<&Intent> {
        self.unresolved
    }

    pub(super) fn owned(&self) -> impl Iterator<Item = &Intent> {
        self.owned.values().filter_map(|stack| stack.last())
    }

    pub(super) fn latest_owned(&self, control: PrivacyControl) -> Option<&Intent> {
        self.owned.get(&control).and_then(|stack| stack.last())
    }
}

impl Fold {
    fn prepare(&mut self, intent: Intent) -> Result<(), HistoryFault> {
        if self.pending.is_some() {
            return Err(HistoryFault::Invalid("an earlier operation is unresolved"));
        }
        if self.latest.is_some_and(|id| id >= intent.operation)
            || self
                .summary
                .as_ref()
                .is_some_and(|summary| summary.owner != intent.owner)
            || intent.original == intent.modified
        {
            return Err(HistoryFault::Invalid("invalid operation identity"));
        }
        if let Some(id) = intent.restore_of {
            let top = self
                .owned
                .get(&intent.control)
                .and_then(|stack| stack.last())
                .ok_or(HistoryFault::Invalid("restore is not owned"))?;
            if top.operation != id
                || top.modified != intent.original
                || top.original != intent.modified
            {
                return Err(HistoryFault::Invalid(
                    "restore does not reverse the latest owned operation of its control",
                ));
            }
        }
        self.latest = Some(intent.operation);
        let status = Status {
            operation: intent.operation,
            outcome: StatusOutcome::Unresolved,
        };
        match &mut self.summary {
            Some(summary) => summary.statuses.push(status),
            None => {
                self.summary = Some(Summary {
                    owner: intent.owner.clone(),
                    statuses: vec![status],
                });
            }
        }
        self.pending = Some(Pending {
            intent,
            stage: Stage::Open,
        });
        Ok(())
    }

    fn finish(&mut self, operation: NonZeroU64, outcome: Outcome) -> Result<(), HistoryFault> {
        let intent = self.close(operation)?;
        let owns = match (outcome, intent.restore_of) {
            (Outcome::Applied, None) => Ownership::Take,
            (Outcome::Restored, Some(_)) => Ownership::Release,
            (Outcome::NotApplied | Outcome::RolledBack, None | Some(_)) => Ownership::Keep,
            (Outcome::Unknown, None | Some(_)) => {
                self.pending = Some(Pending {
                    intent,
                    stage: Stage::Unknown,
                });
                return Ok(());
            }
            (Outcome::Applied, Some(_)) | (Outcome::Restored, None) => {
                return Err(HistoryFault::Invalid(
                    "receipt has the wrong outcome for its action",
                ));
            }
        };
        self.settle(intent, owns, StatusOutcome::Finished(outcome))
    }

    fn reconcile(
        &mut self,
        operation: NonZeroU64,
        settlement: PrivacySettlement,
    ) -> Result<(), HistoryFault> {
        let pending = self.pending.take().ok_or(HistoryFault::Invalid(
            "reconciliation has no unresolved operation",
        ))?;
        if pending.intent.operation != operation {
            return Err(HistoryFault::Invalid(
                "reconciliation belongs to a different operation",
            ));
        }
        let owns = match (settlement, pending.intent.restore_of) {
            (PrivacySettlement::Applied, None) => Ownership::Take,
            (PrivacySettlement::Restored, Some(_)) => Ownership::Release,
            (PrivacySettlement::NotApplied | PrivacySettlement::Abandoned, None | Some(_)) => {
                Ownership::Keep
            }
            (PrivacySettlement::Applied, Some(_)) | (PrivacySettlement::Restored, None) => {
                return Err(HistoryFault::Invalid(
                    "reconciliation has the wrong settlement for its action",
                ));
            }
        };
        self.settle(pending.intent, owns, StatusOutcome::Reconciled(settlement))
    }

    fn close(&mut self, operation: NonZeroU64) -> Result<Intent, HistoryFault> {
        let pending = self
            .pending
            .take()
            .ok_or(HistoryFault::Invalid("receipt has no prepared intent"))?;
        if pending.intent.operation != operation {
            return Err(HistoryFault::Invalid(
                "receipt belongs to a different operation",
            ));
        }
        match pending.stage {
            Stage::Open => Ok(pending.intent),
            Stage::Unknown => Err(HistoryFault::Invalid(
                "an unknown operation ends only by reconciliation",
            )),
        }
    }

    fn settle(
        &mut self,
        intent: Intent,
        owns: Ownership,
        outcome: StatusOutcome,
    ) -> Result<(), HistoryFault> {
        match owns {
            Ownership::Take => self.owned.entry(intent.control).or_default().push(intent),
            Ownership::Release => {
                self.owned
                    .get_mut(&intent.control)
                    .and_then(Vec::pop)
                    .ok_or(HistoryFault::Invalid("restore is not owned"))?;
            }
            Ownership::Keep => (),
        }
        let status = self
            .summary
            .as_mut()
            .and_then(|summary| summary.statuses.last_mut())
            .ok_or(HistoryFault::Invalid("receipt has no status"))?;
        if status.outcome != StatusOutcome::Unresolved {
            return Err(HistoryFault::Invalid("operation already has a receipt"));
        }
        status.outcome = outcome;
        Ok(())
    }
}

enum Ownership {
    Take,
    Release,
    Keep,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
pub(super) mod fixtures;

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests;
