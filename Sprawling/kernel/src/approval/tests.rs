// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

/// The two residents the table talks about: the appointed delegate, and
/// anybody else.
const DELEGATE: &str = "judge@b.1";
const BYSTANDER: &str = "other@b.9";

fn question(actor: &str) -> ApprovalItem {
    ApprovalItem {
        id: ApprovalId::new("item-1").unwrap(),
        actor: actor.into(),
        action_desc: "which of the two schemas should the importer read".into(),
        artifact: Locator::parse(&format!("cas:b3-{}", "ef".repeat(32))).unwrap(),
        cluster_key: ClusterKey {
            class: ApprovalClass::Question,
            detail: "importer:schema".into(),
        },
        created: TimeMs::new(1_000),
        tainted: false,
    }
}

fn resident(name: &str) -> ResidentId {
    ResidentId::new(name).unwrap()
}

/// Who is being asked to answer, spelled so a row of the table reads as
/// a sentence.
#[derive(Debug, Clone, Copy)]
enum Who {
    Person,
    TheDelegate,
    SomebodyElse,
}

fn answerer(who: Who) -> Answerer {
    match who {
        Who::Person => Answerer::Human,
        Who::TheDelegate => Answerer::Resident(resident(DELEGATE)),
        Who::SomebodyElse => Answerer::Resident(resident(BYSTANDER)),
    }
}

/// The truth table of [`may_answer`], every row asserted.
///
/// Twelve rows: two autonomy states, three answerers, and the two
/// actors that matter — the answerer itself, and somebody else. A row
/// added to the table is a row asserted, which is the property this
/// test exists for: the rule may not grow a case that nothing checks.
#[test]
fn the_answer_table_holds_row_by_row() {
    let delegated = Autonomy::Delegate(resident(DELEGATE));
    let table: [(&Autonomy, Who, &str, AnswerVerdict); 12] = [
        // A person answers everything, whoever raised it, under either
        // autonomy: appointing a delegate hands the inbox over, it does
        // not take it away.
        (&Autonomy::Owner, Who::Person, DELEGATE, AnswerVerdict::May),
        (&Autonomy::Owner, Who::Person, BYSTANDER, AnswerVerdict::May),
        (&delegated, Who::Person, DELEGATE, AnswerVerdict::May),
        (&delegated, Who::Person, BYSTANDER, AnswerVerdict::May),
        // Owner mode: no resident is appointed, so no resident answers.
        (
            &Autonomy::Owner,
            Who::TheDelegate,
            BYSTANDER,
            AnswerVerdict::NotTheDelegate,
        ),
        (
            &Autonomy::Owner,
            Who::TheDelegate,
            DELEGATE,
            AnswerVerdict::NotTheDelegate,
        ),
        (
            &Autonomy::Owner,
            Who::SomebodyElse,
            DELEGATE,
            AnswerVerdict::NotTheDelegate,
        ),
        (
            &Autonomy::Owner,
            Who::SomebodyElse,
            BYSTANDER,
            AnswerVerdict::NotTheDelegate,
        ),
        // Delegate mode: the appointed resident answers what somebody
        // else asked, and nobody else's ruling counts.
        (&delegated, Who::TheDelegate, BYSTANDER, AnswerVerdict::May),
        (
            &delegated,
            Who::SomebodyElse,
            DELEGATE,
            AnswerVerdict::NotTheDelegate,
        ),
        (
            &delegated,
            Who::SomebodyElse,
            BYSTANDER,
            AnswerVerdict::NotTheDelegate,
        ),
        // Self-approval is no approval, even for the delegate.
        (
            &delegated,
            Who::TheDelegate,
            DELEGATE,
            AnswerVerdict::SelfApprovalBarred,
        ),
    ];
    for (autonomy, who, actor, expected) in table {
        assert_eq!(
            may_answer(autonomy, &question(actor), &answerer(who)),
            expected,
            "autonomy {autonomy:?}, answerer {who:?}, raised by {actor}"
        );
    }
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
            &question("lab/room1"),
            &Answerer::Resident(clerk.clone())
        ),
        AnswerVerdict::May
    );
    // The Mayor is not the clerk, whatever else it may do.
    assert_eq!(
        may_answer(
            &autonomy,
            &question("lab/room1"),
            &Answerer::Resident(resident(crate::consts_policy::HALL_MAYOR))
        ),
        AnswerVerdict::NotTheDelegate
    );
    // And the clerk does not answer for itself.
    assert_eq!(
        may_answer(
            &autonomy,
            &question(crate::consts_policy::HALL_CLERK),
            &Answerer::Resident(clerk)
        ),
        AnswerVerdict::SelfApprovalBarred
    );
}

#[test]
fn two_runs_raising_their_first_question_get_two_ids() {
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
