// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn item(class: ApprovalClass, tainted: bool, actor: &str) -> ApprovalItem {
    ApprovalItem {
        id: ApprovalId::new("item-1").unwrap(),
        source: ApprovalSource::Gate,
        actor: actor.into(),
        action_desc: "send the release mail".into(),
        artifact: Locator::parse(&format!("cas:b3-{}", "ef".repeat(32))).unwrap(),
        cluster_key: ClusterKey {
            class,
            detail: "mail:release".into(),
        },
        created: TimeMs::new(1_000),
        tainted,
    }
}

fn policy(prefix: &str) -> Policy {
    Policy {
        id: "p-1".into(),
        matcher: PolicyMatcher {
            class: PolicyClass::AgentQuestion,
            detail_prefix: prefix.into(),
        },
        verdict: PolicyVerdict::Allow,
        source: ApprovalId::new("item-0").unwrap(),
        created: TimeMs::new(0),
        last_hit: None,
    }
}

fn resident(name: &str) -> ResidentId {
    ResidentId::new(name).unwrap()
}

#[test]
fn policies_match_class_and_prefix_but_never_tainted_items() {
    let p = policy("mail:");
    assert_eq!(
        match_item(&p, &item(ApprovalClass::AgentQuestion, false, "a")),
        PolicyApplication::Applies(PolicyVerdict::Allow)
    );
    assert_eq!(
        match_item(&p, &item(ApprovalClass::AgentQuestion, true, "a")),
        PolicyApplication::NotApplicable
    );
    assert_eq!(
        match_item(
            &policy("web:"),
            &item(ApprovalClass::AgentQuestion, false, "a")
        ),
        PolicyApplication::NotApplicable
    );
    // Commitment/BudgetLimit/DiscardEscalate: no PolicyClass variant
    // exists to even write such a matcher — pinned by a trybuild case.
    assert_eq!(
        match_item(&p, &item(ApprovalClass::Commitment, false, "a")),
        PolicyApplication::NotApplicable
    );
}

#[test]
fn idle_policies_expire_and_skewed_clocks_do_not() {
    const NINETY_DAYS_MS: u64 = 90 * 86_400_000;
    let mut p = policy("");
    assert_eq!(expiry(&p, TimeMs::new(0)), PolicyExpiry::Active);
    assert_eq!(
        expiry(&p, TimeMs::new(NINETY_DAYS_MS)),
        PolicyExpiry::Expired
    );
    p.last_hit = Some(TimeMs::new(NINETY_DAYS_MS));
    assert_eq!(
        expiry(&p, TimeMs::new(NINETY_DAYS_MS.saturating_add(10))),
        PolicyExpiry::Active
    );
    // now before created: skew reads Active, never a panic.
    let skewed = Policy {
        created: TimeMs::new(100),
        ..policy("")
    };
    assert_eq!(expiry(&skewed, TimeMs::new(50)), PolicyExpiry::Active);
}

#[test]
fn the_answer_matrix_holds() {
    let delegate = resident("judge@b.1");
    let autonomy = Autonomy::Delegate(delegate.clone());
    let question = item(ApprovalClass::AgentQuestion, false, "worker@b.2");
    // Humans always may.
    assert_eq!(
        may_answer(&Autonomy::Owner, &question, &Answerer::Human),
        AnswerVerdict::May
    );
    // The appointed delegate may answer an ordinary question.
    assert_eq!(
        may_answer(&autonomy, &question, &Answerer::Resident(delegate.clone())),
        AnswerVerdict::May
    );
    // Not appointed: someone else's ruling counts for nothing.
    assert_eq!(
        may_answer(
            &autonomy,
            &question,
            &Answerer::Resident(resident("other@b.9"))
        ),
        AnswerVerdict::NotTheDelegate
    );
    // Owner mode: no resident is appointed.
    assert_eq!(
        may_answer(
            &Autonomy::Owner,
            &question,
            &Answerer::Resident(delegate.clone())
        ),
        AnswerVerdict::NotTheDelegate
    );
    // The three classes and tainted items are human-only.
    for class in [
        ApprovalClass::Commitment,
        ApprovalClass::BudgetLimit,
        ApprovalClass::DiscardEscalate,
    ] {
        assert_eq!(
            may_answer(
                &autonomy,
                &item(class, false, "worker@b.2"),
                &Answerer::Resident(delegate.clone())
            ),
            AnswerVerdict::HumanOnly
        );
    }
    assert_eq!(
        may_answer(
            &autonomy,
            &item(ApprovalClass::AgentQuestion, true, "worker@b.2"),
            &Answerer::Resident(delegate.clone())
        ),
        AnswerVerdict::HumanOnly
    );
    // Self-approval is no approval.
    assert_eq!(
        may_answer(
            &autonomy,
            &item(ApprovalClass::AgentQuestion, false, "judge@b.1"),
            &Answerer::Resident(delegate)
        ),
        AnswerVerdict::SelfApprovalBarred
    );
}

/// The clerk is a delegate like any other: nothing about `hall/clerk`
/// is a new rule in this module, which is why no `Autonomy` variant
/// exists for it.
#[test]
fn the_clerk_answers_as_the_appointed_delegate_and_no_further() {
    let clerk = resident(crate::consts_policy::HALL_CLERK);
    let autonomy = Autonomy::Delegate(clerk.clone());
    assert_eq!(
        may_answer(
            &autonomy,
            &item(ApprovalClass::AgentQuestion, false, "lab/room1"),
            &Answerer::Resident(clerk.clone())
        ),
        AnswerVerdict::May
    );
    // The three must-pass-a-human classes stay the person's.
    assert_eq!(
        may_answer(
            &autonomy,
            &item(ApprovalClass::BudgetLimit, false, "lab/room1"),
            &Answerer::Resident(clerk.clone())
        ),
        AnswerVerdict::HumanOnly
    );
    // A tainted item never reaches the clerk (C15).
    assert_eq!(
        may_answer(
            &autonomy,
            &item(ApprovalClass::AgentQuestion, true, "lab/room1"),
            &Answerer::Resident(clerk.clone())
        ),
        AnswerVerdict::HumanOnly
    );
    // The Mayor is not the clerk, whatever else it may do.
    assert_eq!(
        may_answer(
            &autonomy,
            &item(ApprovalClass::AgentQuestion, false, "lab/room1"),
            &Answerer::Resident(resident(crate::consts_policy::HALL_MAYOR))
        ),
        AnswerVerdict::NotTheDelegate
    );
    // And the clerk does not answer for itself.
    assert_eq!(
        may_answer(
            &autonomy,
            &item(
                ApprovalClass::AgentQuestion,
                false,
                crate::consts_policy::HALL_CLERK
            ),
            &Answerer::Resident(clerk)
        ),
        AnswerVerdict::SelfApprovalBarred
    );
}

#[test]
fn two_runs_raising_their_first_approval_get_two_ids() {
    // What the clock-shaped identity lost: the two lanes reach here in
    // the same millisecond, and the inbox keys on this string.
    let left = RunId::from_bytes([1; 16]);
    let right = RunId::from_bytes([2; 16]);
    assert_ne!(
        ApprovalId::of(&left, Seq::FIRST),
        ApprovalId::of(&right, Seq::FIRST)
    );
    assert_ne!(ApprovalId::of(&left, Seq::FIRST).as_str(), "");
}

#[test]
fn replaying_one_run_re_derives_the_same_ids() {
    let run = RunId::from_bytes([7; 16]);
    let first: Vec<ApprovalId> = (0..4)
        .map(|at| ApprovalId::of(&run, Seq::new(at)))
        .collect();
    let again: Vec<ApprovalId> = (0..4)
        .map(|at| ApprovalId::of(&run, Seq::new(at)))
        .collect();
    assert_eq!(first, again);
    // Positions inside one run are distinct, and they sort the way the
    // run raised them.
    let mut sorted = first.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted, first);
}

#[test]
fn the_sweep_slot_belongs_to_its_run_and_to_no_call_position() {
    let run = RunId::from_bytes([9; 16]);
    let other = RunId::from_bytes([10; 16]);
    assert_ne!(ApprovalId::of_sweep(&run), ApprovalId::of_sweep(&other));
    assert_ne!(
        ApprovalId::of_sweep(&run),
        ApprovalId::of(&run, Seq::new(0))
    );
    assert_ne!(
        ApprovalId::of_sweep(&run),
        ApprovalId::of(&run, Seq::new(u64::MAX - 1))
    );
}
