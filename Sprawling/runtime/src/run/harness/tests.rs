// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::Who;
use kernel::event::record::CheckpointFinding;
use kernel::ledger::chain_hash;
use kernel::{Address, B3Hash, EventDraft, GENESIS_PREV, Locator, RunId, Seq};

use super::*;

struct Kept {
    lines: Vec<EventDraft>,
    refs: Vec<EventRef>,
    next: Seq,
    prev: B3Hash,
}

impl Kept {
    fn new() -> Kept {
        Kept {
            lines: Vec::new(),
            refs: Vec::new(),
            next: Seq::FIRST,
            prev: GENESIS_PREV,
        }
    }

    fn kinds(&self) -> Vec<EventKind> {
        self.lines.iter().map(|line| line.kind).collect()
    }
}

impl Ledger for Kept {
    fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError> {
        let record = kernel::EventRecord::from_draft(draft.clone(), self.next, self.prev);
        self.prev = chain_hash(&record.canonical_line()?);
        self.next = self.next.next()?;
        self.lines.push(draft);
        self.refs.push(record.to_ref());
        Ok(record.to_ref())
    }
}

struct Owned {
    addr: Address,
    job: Locator,
    by: Who,
}

fn owned() -> Owned {
    let addr = Address::parse("lab/room1").unwrap();
    let job = Locator::parse(&format!("file:{}/JOB.md@{}", addr.as_str(), "a".repeat(40))).unwrap();
    Owned {
        addr,
        job,
        by: Who::Person,
    }
}

fn charter(owned: &Owned) -> Charter<'_> {
    Charter {
        pictures: &[],
        run: RunId::from_bytes([7; 16]),
        who: "lab/room1",
        addr: &owned.addr,
        task: "close the loop",
        goal: "the harness answers",
        job: &owned.job,
        parent: None,
        predecessor: None,
        dispatched_by: &owned.by,
        routing: None,
        skills: &[],
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        naming: None,
        opening: None,
        generation: None,
        agent: None,
        effort: None,
    }
}

fn handoff(owned: &Owned) -> Handoff {
    Handoff::new(
        vec![owned.job.clone()],
        "close the loop".to_owned(),
        "not recorded".to_owned(),
        "a harness ran it".to_owned(),
        "not recorded".to_owned(),
    )
    .unwrap()
}

fn answered(stop: HarnessStop, text: &str, cut: Option<Cut>) -> (Completion, EventRef) {
    let owned = owned();
    let mut ledger = Kept::new();
    let mut clock = 0u64;
    let mut now = || {
        clock = clock.saturating_add(1);
        Ok(TimeMs::new(clock))
    };
    let mut run = HarnessRun::open(charter(&owned), &mut ledger, &mut now).unwrap();
    if let Some(cut) = cut {
        run.cancel(&mut ledger, cut, TimeMs::new(10)).unwrap();
    }
    let completion = run
        .conclude(
            &mut ledger,
            Conclusion {
                committed: Payload::empty(),
                answered: HarnessAnswered {
                    stop,
                    text: text.to_owned(),
                },
                handoff: &handoff(&owned),
            },
            TimeMs::new(20),
        )
        .unwrap();
    let cited = ledger
        .refs
        .iter()
        .find(|at| at.kind() == EventKind::HarnessAnswered)
        .copied();
    (completion, cited.unwrap())
}

#[test]
fn an_answer_freezes_as_its_stop_and_the_first_cut_say() {
    let done = |text: &str, cut| {
        let (completion, cited) = answered(HarnessStop::EndTurn, text, cut);
        (
            completion,
            Completion::Done(Evidence::new(vec![cited]).unwrap()),
        )
    };
    for (said, cut) in [("fixed", None), ("fixed", Some(Cut::Halt))] {
        let (froze, expected) = done(said, cut);
        assert_eq!(froze, expected, "end_turn with words, cut {cut:?}");
    }
    let rows = [
        (HarnessStop::EndTurn, " \n", None, Completion::Limit),
        (
            HarnessStop::Cancelled,
            "",
            Some(Cut::Deadline),
            Completion::Limit,
        ),
        (
            HarnessStop::Cancelled,
            "half",
            Some(Cut::Halt),
            Completion::Cancelled,
        ),
        (HarnessStop::Cancelled, "", None, Completion::Cancelled),
        (HarnessStop::MaxTokens, "cut off", None, Completion::Limit),
        (HarnessStop::MaxTurnRequests, "", None, Completion::Limit),
        (
            HarnessStop::Refusal,
            "no",
            Some(Cut::Halt),
            Completion::Limit,
        ),
    ];
    for (stop, text, cut, expected) in rows {
        assert_eq!(
            answered(stop, text, cut).0,
            expected,
            "{stop:?} saying {text:?}, cut {cut:?}"
        );
    }
}

#[test]
fn a_concluded_run_cancels_once_and_commits_before_it_answers() {
    let owned = owned();
    let mut ledger = Kept::new();
    let mut clock = 0u64;
    let mut now = || {
        clock = clock.saturating_add(1);
        Ok(TimeMs::new(clock))
    };
    let mut run = HarnessRun::open(charter(&owned), &mut ledger, &mut now).unwrap();
    let said = HarnessReported::Said {
        text: "reading".to_owned(),
    };
    run.report(&mut ledger, &said, TimeMs::new(3)).unwrap();
    run.cancel(&mut ledger, Cut::Deadline, TimeMs::new(4))
        .unwrap();
    run.cancel(&mut ledger, Cut::Halt, TimeMs::new(5)).unwrap();
    let completion = run
        .conclude(
            &mut ledger,
            Conclusion {
                committed: Payload::empty(),
                answered: HarnessAnswered {
                    stop: HarnessStop::Cancelled,
                    text: String::new(),
                },
                handoff: &handoff(&owned),
            },
            TimeMs::new(6),
        )
        .unwrap();

    assert_eq!(
        ledger.kinds(),
        vec![
            EventKind::CheckpointCommitted,
            EventKind::RunStarted,
            EventKind::HarnessReported,
            EventKind::CancelReceived,
            EventKind::CheckpointCommitted,
            EventKind::HarnessAnswered,
            EventKind::HandoffWritten,
            EventKind::RunFrozen,
        ]
    );
    assert_eq!(
        completion,
        Completion::Limit,
        "the first cut was the ceiling, and the second one did not replace it"
    );
}

#[test]
fn a_lost_session_freezes_cancelled_with_no_answer() {
    let owned = owned();
    let mut ledger = Kept::new();
    let mut clock = 0u64;
    let mut now = || {
        clock = clock.saturating_add(1);
        Ok(TimeMs::new(clock))
    };
    let run = HarnessRun::open(charter(&owned), &mut ledger, &mut now).unwrap();
    let completion = run
        .abandon(&mut ledger, &handoff(&owned), None, TimeMs::new(9))
        .unwrap();

    assert_eq!(completion, Completion::Cancelled);
    assert_eq!(
        ledger.kinds(),
        vec![
            EventKind::CheckpointCommitted,
            EventKind::RunStarted,
            EventKind::HandoffWritten,
            EventKind::RunFrozen,
        ]
    );
}

#[test]
fn a_refused_tree_is_booked_before_the_run_freezes() {
    let owned = owned();
    let mut ledger = Kept::new();
    let mut clock = 0u64;
    let mut now = || {
        clock = clock.saturating_add(1);
        Ok(TimeMs::new(clock))
    };
    let run = HarnessRun::open(charter(&owned), &mut ledger, &mut now).unwrap();
    let report = SecretEgressBlocked::checkpoint(
        vec!["lab".to_owned()],
        vec![CheckpointFinding {
            path: "lab/SPEC.md".to_owned(),
            offset: 4,
            len: 30,
            line: 1,
            reason: "anthropic".to_owned(),
        }],
    );
    run.refusal(&mut ledger, &report, TimeMs::new(5)).unwrap();
    run.abandon(&mut ledger, &handoff(&owned), None, TimeMs::new(6))
        .unwrap();

    assert_eq!(
        ledger.kinds(),
        vec![
            EventKind::CheckpointCommitted,
            EventKind::RunStarted,
            EventKind::SecretEgressBlocked,
            EventKind::HandoffWritten,
            EventKind::RunFrozen,
        ]
    );
    let booked = ledger
        .lines
        .iter()
        .find(|line| line.kind == EventKind::SecretEgressBlocked)
        .unwrap();
    assert_eq!(booked.data.as_map()["code"], "E_SECRET_EGRESS");
    assert_eq!(booked.data.as_map()["withheld"][0]["reason"], "anthropic");
}
