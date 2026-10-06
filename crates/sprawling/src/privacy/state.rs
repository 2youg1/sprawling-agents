// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Exact history values and their fold (`crates/sprawling/spec/Privacy/State.lean`).

use std::num::NonZeroU64;

use kernel::{AxCode, AxError, SecretRef};
use serde::{Deserialize, Serialize};

pub(super) const SCHEMA: u32 = 2;
pub(super) const DEFINITION: u32 = 1;

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum RawValue {
    Absent { key_existed: bool },
    Present { kind: u32, bytes: Vec<u8> },
}

#[derive(Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Control {
    WindowsUserPowershellTelemetry,
}

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Intent {
    pub(super) operation: NonZeroU64,
    pub(super) control: Control,
    pub(super) definition: u32,
    pub(super) owner: SecretRef,
    pub(super) original: RawValue,
    pub(super) modified: RawValue,
    pub(super) recommendation: RawValue,
    pub(super) restore_of: Option<NonZeroU64>,
}

#[derive(Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Outcome {
    Applied,
    NotApplied,
    Restored,
    Unknown,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "phase", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Event {
    Prepared {
        intent: Intent,
    },
    Finished {
        operation: NonZeroU64,
        outcome: Outcome,
    },
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Line {
    pub(super) schema: u32,
    pub(super) event: Event,
}

#[derive(PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum StatusOutcome {
    Unresolved,
    Finished(Outcome),
}

#[derive(PartialEq, Eq, Serialize)]
pub(super) struct Status {
    operation: NonZeroU64,
    outcome: StatusOutcome,
}

#[derive(Default)]
pub(super) struct History {
    statuses: Vec<Status>,
    owner: Option<SecretRef>,
}

impl History {
    pub(super) fn fold(lines: Vec<Line>) -> Result<Self, HistoryFault> {
        let mut history = Self::default();
        let mut owned: Vec<Intent> = Vec::new();
        let mut pending: Option<Intent> = None;
        let mut latest = None;
        for line in lines {
            if line.schema != SCHEMA {
                return Err(HistoryFault::Invalid("unknown history schema"));
            }
            match line.event {
                Event::Prepared { intent } => {
                    if pending.is_some() {
                        return Err(HistoryFault::Invalid("an earlier operation is unresolved"));
                    }
                    if latest.is_some_and(|id| id >= intent.operation)
                        || intent.definition != DEFINITION
                        || history
                            .owner
                            .as_ref()
                            .is_some_and(|previous| previous != &intent.owner)
                        || intent.original == intent.modified
                    {
                        return Err(HistoryFault::Invalid(
                            "invalid operation identity or definition",
                        ));
                    }
                    if let Some(id) = intent.restore_of {
                        let old = owned
                            .last()
                            .ok_or(HistoryFault::Invalid("restore is not owned"))?;
                        if old.operation != id
                            || old.owner != intent.owner
                            || old.control != intent.control
                            || old.definition != intent.definition
                            || old.modified != intent.original
                            || old.original != intent.modified
                        {
                            return Err(HistoryFault::Invalid(
                                "restore does not reverse the latest owned operation",
                            ));
                        }
                    } else if intent.modified != intent.recommendation {
                        return Err(HistoryFault::Invalid(
                            "apply differs from its recommendation",
                        ));
                    }
                    history.owner = Some(intent.owner.clone());
                    latest = Some(intent.operation);
                    history.statuses.push(Status {
                        operation: intent.operation,
                        outcome: StatusOutcome::Unresolved,
                    });
                    pending = Some(intent);
                }
                Event::Finished { operation, outcome } => {
                    let intent = pending
                        .take()
                        .ok_or(HistoryFault::Invalid("receipt has no prepared intent"))?;
                    if operation != intent.operation {
                        return Err(HistoryFault::Invalid(
                            "receipt belongs to a different operation",
                        ));
                    }
                    match (outcome, intent.restore_of) {
                        (Outcome::Applied, None) => owned.push(intent),
                        (Outcome::Restored, Some(_)) => {
                            owned
                                .pop()
                                .ok_or(HistoryFault::Invalid("restore is not owned"))?;
                        }
                        (Outcome::NotApplied, None | Some(_)) => (),
                        (Outcome::Unknown, None | Some(_)) => pending = Some(intent),
                        (Outcome::Applied, Some(_)) | (Outcome::Restored, None) => {
                            return Err(HistoryFault::Invalid(
                                "receipt has the wrong outcome for its action",
                            ));
                        }
                    }
                    let status = history
                        .statuses
                        .last_mut()
                        .ok_or(HistoryFault::Invalid("receipt has no status"))?;
                    if status.outcome != StatusOutcome::Unresolved {
                        return Err(HistoryFault::Invalid("operation already has a receipt"));
                    }
                    status.outcome = StatusOutcome::Finished(outcome);
                }
            }
        }
        Ok(history)
    }

    pub(super) fn owner(&self) -> Option<&SecretRef> {
        self.owner.as_ref()
    }

    pub(super) fn statuses(&self) -> &[Status] {
        &self.statuses
    }
}

/// Failure reasons remain distinct until the CLI boundary maps them to AxError.
#[derive(Debug)]
pub(super) enum HistoryFault {
    Io(std::io::Error),
    Decode {
        line: usize,
        column: usize,
        category: serde_json::error::Category,
    },
    Invalid(&'static str),
    Busy,
    Capacity,
}

impl HistoryFault {
    pub(super) fn into_ax(self) -> AxError {
        let (code, subject, recovery) = match self {
            Self::Io(source) => (
                AxCode::StorageFatal,
                format!("history IO: {source}"),
                "check local history access and retry the read",
            ),
            Self::Decode {
                line,
                column,
                category,
            } => (
                AxCode::StorageFatal,
                format!("history decoding at line {line} column {column} ({category:?})"),
                "preserve the history bytes and inspect the malformed line; do not truncate it",
            ),
            Self::Invalid(reason) => (
                AxCode::StorageFatal,
                reason.to_owned(),
                "preserve the history and resolve its incompatible or unresolved operation before any write",
            ),
            Self::Busy => (
                AxCode::LedgerHeld,
                "privacy history is held by another process".to_owned(),
                "wait for that operation to end, then read status again",
            ),
            Self::Capacity => (
                AxCode::StorageFatal,
                "privacy history exceeds its reader capacity".to_owned(),
                "preserve the history and use a reader that supports its size; do not remove original values",
            ),
        };
        AxError::failure(code, "read local privacy history", subject).with_recovery(recovery)
    }
}
