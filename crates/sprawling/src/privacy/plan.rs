// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The apply, restore, readback and reconciliation decisions
//! (`crates/sprawling/spec/Privacy.lean` §8–9): `planApply`, `planRestore`,
//! `judgeReadback`, `settle` and `reconciled`, over a history whose owner
//! was already checked and a fresh read the coordinator took. No IO and no
//! clock; the coordinator carries out what these return.

use std::num::NonZeroU64;

use wire::PrivacyControl;

use super::controls::definition;
use super::state::{Holdings, Intent, Settlement};
use super::target::{Reading, Snapshot};

/// One apply or restore, as the coordinator accepted it: the control, the
/// number the operation takes, and the snapshot the person confirmed.
pub(super) struct Request<'r> {
    pub(super) control: PrivacyControl,
    pub(super) operation: NonZeroU64,
    pub(super) expected: &'r Snapshot,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum ApplyPlan {
    Write(Intent),
    /// An earlier operation has no conclusion; nothing is written until
    /// the person reconciles it.
    Unresolved,
    /// The target no longer reads what the person confirmed.
    Changed,
    /// The host has nothing this control can write (a task it lacks).
    TargetAbsent,
    /// The target already reads the written value; nothing is recorded
    /// and nothing is owned.
    AlreadyWritten,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum RestorePlan {
    Write(Intent),
    Unresolved,
    /// This app owns no change of the control.
    NothingOwned,
    Changed,
    /// The target no longer reads what this app wrote; someone else's
    /// later value is not overwritten.
    Conflict,
}

/// How a readback compares with what an operation set out to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Readback {
    Matches,
    StillOriginal,
    Other,
}

/// What a readback concludes; `RollBack` concludes nothing yet, because
/// the original has to be written back first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Verdict {
    Applied,
    Restored,
    NotApplied,
    RollBack,
}

/// How a rollback ended, judged by the read that follows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RollbackEnd {
    RolledBack,
    /// The target reads neither value this operation knows: unknown.
    Lost,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum ReconcilePlan {
    Settle(Settlement),
    Changed,
}

/// `planApply` after the identity check (Privacy D66).
pub(super) fn plan_apply(
    holdings: &Holdings<'_>,
    request: &Request<'_>,
    fresh: &Reading,
) -> ApplyPlan {
    if holdings.unresolved().is_some() {
        return ApplyPlan::Unresolved;
    }
    if *request.expected != fresh.value {
        return ApplyPlan::Changed;
    }
    match definition(request.control).written.target(&fresh.value) {
        None => ApplyPlan::TargetAbsent,
        Some(target) if target == fresh.value => ApplyPlan::AlreadyWritten,
        Some(target) => ApplyPlan::Write(Intent {
            operation: request.operation,
            control: request.control,
            owner: holdings.owner().clone(),
            original: fresh.value.clone(),
            modified: target,
            key_existed: fresh.key_existed,
            restore_of: None,
        }),
    }
}

/// `planRestore` after the identity check: the restore reverses the top of
/// the control's own stack, and only while the target still reads what that
/// operation wrote (Privacy D51).
pub(super) fn plan_restore(
    holdings: &Holdings<'_>,
    request: &Request<'_>,
    fresh: &Reading,
) -> RestorePlan {
    if holdings.unresolved().is_some() {
        return RestorePlan::Unresolved;
    }
    let Some(top) = holdings.latest_owned(request.control) else {
        return RestorePlan::NothingOwned;
    };
    if *request.expected != fresh.value {
        return RestorePlan::Changed;
    }
    if fresh.value != top.modified {
        return RestorePlan::Conflict;
    }
    RestorePlan::Write(Intent {
        operation: request.operation,
        control: request.control,
        owner: holdings.owner().clone(),
        original: fresh.value.clone(),
        modified: top.original.clone(),
        key_existed: fresh.key_existed,
        restore_of: Some(top.operation),
    })
}

pub(super) fn judge_readback(intent: &Intent, value: &Snapshot) -> Readback {
    if *value == intent.modified {
        Readback::Matches
    } else if *value == intent.original {
        Readback::StillOriginal
    } else {
        Readback::Other
    }
}

pub(super) fn settle(intent: &Intent, readback: Readback) -> Verdict {
    match (readback, intent.restore_of) {
        (Readback::Matches, None) => Verdict::Applied,
        (Readback::Matches, Some(_)) => Verdict::Restored,
        (Readback::StillOriginal, None | Some(_)) => Verdict::NotApplied,
        (Readback::Other, None | Some(_)) => Verdict::RollBack,
    }
}

/// The model's `rolledBack` and `rollbackLost`: only the original ends a
/// rollback.
pub(super) fn settle_rollback(intent: &Intent, value: &Snapshot) -> RollbackEnd {
    match judge_readback(intent, value) {
        Readback::StillOriginal => RollbackEnd::RolledBack,
        Readback::Matches | Readback::Other => RollbackEnd::Lost,
    }
}

/// The person's check of `unresolved`: the readback rule as usual, a third
/// value abandoned. It writes nothing.
pub(super) fn plan_reconcile(
    unresolved: &Intent,
    expected: &Snapshot,
    fresh: &Snapshot,
) -> ReconcilePlan {
    if expected != fresh {
        return ReconcilePlan::Changed;
    }
    ReconcilePlan::Settle(reconciled(unresolved, fresh))
}

fn reconciled(intent: &Intent, value: &Snapshot) -> Settlement {
    match settle(intent, judge_readback(intent, value)) {
        Verdict::Applied => Settlement::Applied,
        Verdict::Restored => Settlement::Restored,
        Verdict::NotApplied => Settlement::NotApplied,
        Verdict::RollBack => Settlement::Abandoned,
    }
}

/// Replays the vectors `crates/sprawling/spec/Privacy.lean` §11 prints with
/// `#eval`. The model's control 1 writes the DWORD 1, here
/// `feedback_notifications`; its control 3 disables a task, here
/// `ceip_consolidator_task`; its owner 7 is the fixture owner and 8 another.
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, reason = "test code")]
mod tests {
    use super::super::state::fixtures::{apply, dword, finished, operation, owner, prepared};
    use super::super::state::{History, Line, Outcome};
    use super::super::target::{RawValue, TaskState};
    use super::*;
    use kernel::{AxCode, AxError, SecretRef};

    const ONE: PrivacyControl = PrivacyControl::FeedbackNotifications;
    const TASK: PrivacyControl = PrivacyControl::CeipConsolidatorTask;
    const OTHER: PrivacyControl = PrivacyControl::StartLaunchTracking;

    enum Owner {
        Seven,
        Eight,
    }

    fn absent() -> Snapshot {
        Snapshot::Registry(RawValue::Absent)
    }

    fn text() -> Snapshot {
        Snapshot::Registry(RawValue::Present {
            kind: 1,
            bytes: vec![49, 0, 0, 0],
        })
    }

    fn task(state: fn([u8; 32]) -> TaskState) -> Snapshot {
        Snapshot::Task(state([5; 32]))
    }

    fn enabled(definition_sha256: [u8; 32]) -> TaskState {
        TaskState::Enabled { definition_sha256 }
    }

    fn disabled(definition_sha256: [u8; 32]) -> TaskState {
        TaskState::Disabled { definition_sha256 }
    }

    /// The model's `applied`: operation 1 wrote 1 over an absent value.
    fn applied() -> Intent {
        Intent {
            key_existed: false,
            ..apply(1, ONE, absent(), dword(1))
        }
    }

    /// History lines that leave control 1 owning `owned`, with an operation
    /// open when `phase` is not idle. A history the model's owner 8 reads
    /// needs an owner, so `Phase::Idle` with nothing owned still records a
    /// concluded operation of another control.
    fn lines(phase: Phase, owned: &[Intent]) -> Vec<Line> {
        let mut lines = Vec::new();
        for intent in owned {
            lines.extend([prepared(intent), finished(intent, Outcome::Applied)]);
        }
        let earlier = apply(9, OTHER, dword(1), dword(0));
        match phase {
            Phase::Idle if owned.is_empty() => {
                lines.extend([prepared(&earlier), finished(&earlier, Outcome::NotApplied)]);
            }
            Phase::Idle => (),
            Phase::Prepared => lines.push(prepared(&earlier)),
            Phase::Unknown => {
                lines.extend([prepared(&earlier), finished(&earlier, Outcome::Unknown)]);
            }
        }
        lines
    }

    #[derive(Clone, Copy)]
    enum Phase {
        Idle,
        Prepared,
        Unknown,
    }

    fn authorize(who: &Owner) -> impl FnOnce(Option<&SecretRef>) -> Result<SecretRef, AxError> {
        move |recorded| match who {
            Owner::Seven => Ok(recorded.cloned().unwrap_or_else(owner)),
            Owner::Eight => Err(AxError::failure(
                AxCode::ConfigInvalid,
                "verify privacy owner",
                "privacy history belongs to another identity",
            )
            .with_recovery("run as the owner")),
        }
    }

    /// The model's `vectorState` and `request` for one control: `None` is
    /// the model's `notOwner`, which here is the holdings refusal.
    struct Row {
        phase: Phase,
        owned: Vec<Intent>,
        who: Owner,
        control: PrivacyControl,
        live: Snapshot,
        expected: Snapshot,
        key_existed: bool,
    }

    fn row(phase: Phase, who: Owner, control: PrivacyControl, live: Snapshot) -> Row {
        Row {
            phase,
            owned: Vec::new(),
            who,
            control,
            expected: live.clone(),
            live,
            key_existed: false,
        }
    }

    fn decide<P>(row: &Row, plan: fn(&Holdings<'_>, &Request<'_>, &Reading) -> P) -> Option<P> {
        let history = History::fold(lines(row.phase, &row.owned)).unwrap();
        let holdings = history.holdings(authorize(&row.who)).ok()?;
        let request = Request {
            control: row.control,
            operation: operation(2),
            expected: &row.expected,
        };
        let fresh = Reading {
            value: row.live.clone(),
            key_existed: row.key_existed,
        };
        Some(plan(&holdings, &request, &fresh))
    }

    #[test]
    fn apply_vectors_replay() {
        let rows = [
            row(Phase::Idle, Owner::Eight, ONE, absent()),
            row(Phase::Unknown, Owner::Seven, ONE, absent()),
            Row {
                expected: absent(),
                ..row(Phase::Idle, Owner::Seven, ONE, dword(0))
            },
            row(Phase::Idle, Owner::Seven, ONE, dword(1)),
            row(Phase::Idle, Owner::Seven, ONE, absent()),
            Row {
                key_existed: true,
                ..row(Phase::Idle, Owner::Seven, ONE, text())
            },
            row(
                Phase::Idle,
                Owner::Seven,
                TASK,
                Snapshot::Task(TaskState::Absent),
            ),
            row(Phase::Idle, Owner::Seven, TASK, task(disabled)),
            row(Phase::Idle, Owner::Seven, TASK, task(enabled)),
        ];
        let write = |control, original, modified, key_existed| {
            Some(ApplyPlan::Write(Intent {
                key_existed,
                ..apply(2, control, original, modified)
            }))
        };
        assert_eq!(
            rows.iter()
                .map(|row| decide(row, plan_apply))
                .collect::<Vec<_>>(),
            [
                None,
                Some(ApplyPlan::Unresolved),
                Some(ApplyPlan::Changed),
                Some(ApplyPlan::AlreadyWritten),
                write(ONE, absent(), dword(1), false),
                write(ONE, text(), dword(1), true),
                Some(ApplyPlan::TargetAbsent),
                Some(ApplyPlan::AlreadyWritten),
                write(TASK, task(enabled), task(disabled), false),
            ]
        );
    }

    #[test]
    fn restore_vectors_replay() {
        let owning = |phase, who, live: Snapshot, expected| Row {
            owned: vec![applied()],
            expected,
            key_existed: true,
            ..row(phase, who, ONE, live)
        };
        let rows = [
            owning(Phase::Idle, Owner::Eight, dword(1), dword(1)),
            owning(Phase::Prepared, Owner::Seven, dword(1), dword(1)),
            Row {
                key_existed: true,
                ..row(Phase::Idle, Owner::Seven, ONE, dword(1))
            },
            owning(Phase::Idle, Owner::Seven, dword(1), dword(0)),
            owning(Phase::Idle, Owner::Seven, dword(0), dword(0)),
            owning(Phase::Idle, Owner::Seven, dword(1), dword(1)),
        ];
        assert_eq!(
            rows.iter()
                .map(|row| decide(row, plan_restore))
                .collect::<Vec<_>>(),
            [
                None,
                Some(RestorePlan::Unresolved),
                Some(RestorePlan::NothingOwned),
                Some(RestorePlan::Changed),
                Some(RestorePlan::Conflict),
                Some(RestorePlan::Write(Intent {
                    operation: operation(2),
                    original: dword(1),
                    modified: absent(),
                    key_existed: true,
                    restore_of: Some(operation(1)),
                    ..applied()
                })),
            ]
        );
    }

    #[test]
    fn readback_vectors_replay() {
        let restore = Intent {
            operation: operation(2),
            original: dword(1),
            modified: absent(),
            restore_of: Some(operation(1)),
            ..applied()
        };
        let judged: Vec<_> = [applied(), restore]
            .iter()
            .flat_map(|intent| {
                [intent.modified.clone(), intent.original.clone(), dword(0)].map(|value| {
                    let readback = judge_readback(intent, &value);
                    (
                        readback,
                        settle(intent, readback),
                        reconciled(intent, &value),
                    )
                })
            })
            .collect();
        assert_eq!(
            judged,
            [
                (Readback::Matches, Verdict::Applied, Settlement::Applied),
                (
                    Readback::StillOriginal,
                    Verdict::NotApplied,
                    Settlement::NotApplied
                ),
                (Readback::Other, Verdict::RollBack, Settlement::Abandoned),
                (Readback::Matches, Verdict::Restored, Settlement::Restored),
                (
                    Readback::StillOriginal,
                    Verdict::NotApplied,
                    Settlement::NotApplied
                ),
                (Readback::Other, Verdict::RollBack, Settlement::Abandoned),
            ]
        );
    }
}
