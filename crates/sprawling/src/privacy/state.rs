// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Exact history values and their fold (`crates/sprawling/spec/Privacy/State.lean`).

use std::collections::BTreeMap;
use std::num::NonZeroU64;

use kernel::{AxError, SecretRef};
use serde::{Deserialize, Serialize};
use wire::{PrivacyControl, PrivacySettlement};

use super::fault::HistoryFault;
use super::target::Snapshot;

pub(super) const SCHEMA: u32 = 3;

/// What one operation set out to write. Recorded before the host is
/// written and never edited, so `original` is the value a restore puts back.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Intent {
    pub(super) operation: NonZeroU64,
    pub(super) control: PrivacyControl,
    pub(super) owner: SecretRef,
    pub(super) original: Snapshot,
    pub(super) modified: Snapshot,
    /// Whether the parent key existed when `original` was read; reported
    /// as a residual empty key, never compared as part of a value (D58).
    pub(super) key_existed: bool,
    pub(super) restore_of: Option<NonZeroU64>,
}

/// How a write ended, judged by the readback. `Unknown` does not end the
/// operation: only a [`PrivacySettlement`] does.
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

/// The operations of one history and the owner reference every one of
/// them was prepared under, held together so that a summary without an
/// owner cannot be built (`crates/sprawling/spec/Privacy/State.lean`).
#[derive(Debug)]
struct Summary {
    owner: SecretRef,
    statuses: Vec<Status>,
}

/// A folded history. What it holds is released only through
/// [`History::disclose`] and [`History::holdings`], each behind the check
/// of its owner reference.
#[derive(Debug, Default)]
pub(super) struct History(Fold);

/// What a history holds, released once its owner reference passed the
/// identity check (Privacy D66). `O` is what the check returned: the
/// owner reference new intents are prepared under, or `()` for a reader
/// that will not write (Privacy.State).
#[derive(Debug)]
pub(super) struct Holdings<'h, O = SecretRef> {
    owner: O,
    owned: &'h BTreeMap<PrivacyControl, Vec<Intent>>,
    unresolved: Option<&'h Intent>,
    latest: Option<NonZeroU64>,
}

/// The operation still open at the end of the lines read so far.
#[derive(Debug)]
struct Pending {
    intent: Intent,
    stage: Stage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    /// Prepared, and no outcome recorded yet.
    Open,
    /// The readback ended `Unknown`; only a settlement ends it.
    Unknown,
}

/// The fold's working state: each control's ownership stack, the open
/// operation, and the newest operation number seen.
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

    /// The operation summaries, released only once `authorize` accepts the
    /// owner reference they were prepared under. An empty history has no
    /// owner, so `authorize` is not asked and nothing is released.
    ///
    /// # Errors
    /// The refusal `authorize` returns; the history is unchanged.
    pub(super) fn disclose(
        &self,
        authorize: impl FnOnce(&SecretRef) -> Result<(), AxError>,
    ) -> Result<&[Status], AxError> {
        match &self.0.summary {
            None => Ok(&[]),
            Some(summary) => authorize(&summary.owner).map(|()| summary.statuses.as_slice()),
        }
    }

    /// The ownership stacks, the unresolved operation and the next
    /// operation number, released once `authorize` accepts the recorded
    /// owner reference (`None` for an empty history) and returns the owner
    /// every new intent is prepared under, or `()` for a reader.
    ///
    /// # Errors
    /// The refusal `authorize` returns; nothing of the history is released.
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

    /// The number the next operation takes; `None` once the numbers ran out.
    pub(super) fn next_operation(&self) -> Option<NonZeroU64> {
        match self.latest {
            None => Some(NonZeroU64::MIN),
            Some(latest) => latest.checked_add(1),
        }
    }
}

impl<O> Holdings<'_, O> {
    /// The operation that has no conclusion yet: prepared and never
    /// finished, or finished `Unknown` and never reconciled.
    pub(super) fn unresolved(&self) -> Option<&Intent> {
        self.unresolved
    }

    /// The latest operation each control still owns, in page order.
    pub(super) fn owned(&self) -> impl Iterator<Item = &Intent> {
        self.owned.values().filter_map(|stack| stack.last())
    }

    /// The latest operation `control` still owns: the top of its stack.
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

    /// Takes the open operation a receipt names; an operation whose
    /// readback ended unknown takes no second receipt.
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

/// What a conclusion does to its control's ownership stack: only a matched
/// apply takes ownership and only a matched restore releases it.
enum Ownership {
    Take,
    Release,
    Keep,
}

/// History lines for the tests of this module and of its readers.
#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
pub(super) mod fixtures {
    use super::*;
    use crate::privacy::target::RawValue;

    pub(in crate::privacy) fn owner() -> SecretRef {
        SecretRef::new("privacy", "fixture-owner").unwrap()
    }

    pub(in crate::privacy) fn dword(value: u32) -> Snapshot {
        Snapshot::Registry(RawValue::Present {
            kind: 4,
            bytes: value.to_le_bytes().to_vec(),
        })
    }

    pub(in crate::privacy) fn operation(id: u64) -> NonZeroU64 {
        NonZeroU64::new(id).unwrap()
    }

    pub(in crate::privacy) fn apply(
        id: u64,
        control: PrivacyControl,
        original: Snapshot,
        modified: Snapshot,
    ) -> Intent {
        Intent {
            operation: operation(id),
            control,
            owner: owner(),
            original,
            modified,
            key_existed: true,
            restore_of: None,
        }
    }

    /// The restore that reverses `owned`.
    pub(in crate::privacy) fn restore(id: u64, owned: &Intent) -> Intent {
        Intent {
            operation: operation(id),
            original: owned.modified.clone(),
            modified: owned.original.clone(),
            restore_of: Some(owned.operation),
            ..owned.clone()
        }
    }

    pub(in crate::privacy) fn prepared(intent: &Intent) -> Line {
        Line {
            schema: SCHEMA,
            event: Event::Prepared {
                intent: intent.clone(),
            },
        }
    }

    pub(in crate::privacy) fn finished(intent: &Intent, outcome: Outcome) -> Line {
        Line {
            schema: SCHEMA,
            event: Event::Finished {
                operation: intent.operation,
                outcome,
            },
        }
    }

    pub(in crate::privacy) fn reconciled(intent: &Intent, settlement: PrivacySettlement) -> Line {
        Line {
            schema: SCHEMA,
            event: Event::Reconciled {
                operation: intent.operation,
                settlement,
            },
        }
    }

    pub(in crate::privacy) fn bytes(lines: &[Line]) -> Vec<u8> {
        lines
            .iter()
            .flat_map(|line| {
                let mut bytes = serde_json::to_vec(line).unwrap();
                bytes.push(b'\n');
                bytes
            })
            .collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::fixtures::*;
    use super::*;
    use crate::privacy::target::{RawValue, TaskState};

    const TELEMETRY: PrivacyControl = PrivacyControl::DiagnosticData;
    const TRACKING: PrivacyControl = PrivacyControl::StartLaunchTracking;

    fn statuses(lines: Vec<Line>) -> Result<serde_json::Value, HistoryFault> {
        History::fold(lines)
            .map(|history| serde_json::to_value(history.disclose(|_| Ok(())).unwrap()).unwrap())
    }

    /// Each control keeps its own stack (Privacy D51): a restore reverses the
    /// top of its own control's stack whatever other controls did since, and
    /// never the top of another control's stack or a lower layer of its own.
    #[test]
    fn restore_reverses_only_the_top_of_its_own_controls_stack() {
        let telemetry = apply(1, TELEMETRY, Snapshot::Registry(RawValue::Absent), dword(0));
        let tracking = apply(2, TRACKING, dword(1), dword(0));
        let history = [
            prepared(&telemetry),
            finished(&telemetry, Outcome::Applied),
            prepared(&tracking),
            finished(&tracking, Outcome::Applied),
        ];
        let back = restore(3, &telemetry);
        let mut accepted = history.to_vec();
        accepted.extend([prepared(&back), finished(&back, Outcome::Restored)]);
        assert!(History::fold(accepted).is_ok());

        let crossed = Intent {
            control: TRACKING,
            ..back.clone()
        };
        let mut refused = history.to_vec();
        refused.push(prepared(&crossed));
        assert!(History::fold(refused).is_err());

        let second = apply(2, TELEMETRY, dword(0), dword(1));
        let lower = restore(3, &telemetry);
        assert!(
            History::fold(vec![
                prepared(&telemetry),
                finished(&telemetry, Outcome::Applied),
                prepared(&second),
                finished(&second, Outcome::Applied),
                prepared(&lower),
            ])
            .is_err()
        );
    }

    /// Only a matched apply owns: a rolled-back, not-applied or abandoned
    /// operation leaves nothing to restore, and a settlement of `Applied`
    /// owns as a receipt of `Applied` does.
    #[test]
    fn only_a_matched_apply_takes_ownership() {
        for (end, owns) in [
            (vec![Outcome::RolledBack], false),
            (vec![Outcome::NotApplied], false),
            (vec![Outcome::Unknown], false),
            (vec![Outcome::Applied], true),
        ] {
            let intent = apply(1, TELEMETRY, dword(5), dword(0));
            let mut lines = vec![prepared(&intent)];
            lines.extend(end.iter().map(|outcome| finished(&intent, *outcome)));
            lines.push(prepared(&restore(2, &intent)));
            assert_eq!(History::fold(lines).is_ok(), owns, "{end:?}");
        }
        for (settlement, owns) in [
            (PrivacySettlement::Abandoned, false),
            (PrivacySettlement::NotApplied, false),
            (PrivacySettlement::Applied, true),
        ] {
            let intent = apply(1, TELEMETRY, dword(5), dword(0));
            let lines = vec![
                prepared(&intent),
                finished(&intent, Outcome::Unknown),
                reconciled(&intent, settlement),
                prepared(&restore(2, &intent)),
            ];
            assert_eq!(History::fold(lines).is_ok(), owns, "{settlement:?}");
        }
    }

    /// An unknown readback keeps the operation open: status says
    /// unresolved, no second receipt and no new operation is accepted, and
    /// only a reconciliation of that operation ends it.
    #[test]
    fn an_unknown_operation_ends_only_by_its_reconciliation() {
        let intent = apply(1, TELEMETRY, dword(5), dword(0));
        let unknown = vec![prepared(&intent), finished(&intent, Outcome::Unknown)];
        assert_eq!(
            statuses(unknown.clone()).unwrap(),
            serde_json::json!([{ "operation": 1, "outcome": "unresolved" }])
        );
        let next = apply(2, TRACKING, dword(1), dword(0));
        let other = apply(2, TELEMETRY, dword(5), dword(0));
        for refused in [
            finished(&intent, Outcome::Applied),
            prepared(&next),
            reconciled(&other, PrivacySettlement::Abandoned),
        ] {
            let mut lines = unknown.clone();
            lines.push(refused);
            assert!(History::fold(lines).is_err());
        }
        let mut settled = unknown;
        settled.extend([
            reconciled(&intent, PrivacySettlement::Abandoned),
            prepared(&next),
        ]);
        assert_eq!(
            statuses(settled).unwrap(),
            serde_json::json!([
                { "operation": 1, "outcome": { "reconciled": "abandoned" } },
                { "operation": 2, "outcome": "unresolved" }
            ])
        );
        assert!(History::fold(vec![reconciled(&intent, PrivacySettlement::Applied)]).is_err());
    }

    fn snapshots() -> impl proptest::strategy::Strategy<Value = Snapshot> {
        use proptest::prelude::*;
        prop_oneof![
            Just(Snapshot::Registry(RawValue::Absent)),
            (any::<u32>(), proptest::collection::vec(any::<u8>(), 0..512))
                .prop_map(|(kind, bytes)| Snapshot::Registry(RawValue::Present { kind, bytes })),
            Just(Snapshot::Task(TaskState::Absent)),
            any::<[u8; 32]>().prop_map(|definition_sha256| {
                Snapshot::Task(TaskState::Enabled { definition_sha256 })
            }),
            any::<[u8; 32]>().prop_map(|definition_sha256| {
                Snapshot::Task(TaskState::Disabled { definition_sha256 })
            }),
        ]
    }

    proptest::proptest! {
        /// Original values come back exactly as they were written: type
        /// code, every byte, absence, and a task's definition digest.
        #[test]
        fn intents_roundtrip_without_normalization(
            original in snapshots(),
            modified in snapshots(),
            key_existed in proptest::bool::ANY,
        ) {
            let before = Intent { key_existed, ..apply(1, TELEMETRY, original, modified) };
            let decoded: Line = serde_json::from_slice(&bytes(&[prepared(&before)])).unwrap();
            let Event::Prepared { intent } = decoded.event else {
                return Err(proptest::test_runner::TestCaseError::fail("not a prepared line"));
            };
            proptest::prop_assert_eq!(before, intent);
        }
    }
}
