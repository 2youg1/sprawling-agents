// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The derived checks of `spec/Delivery.lean` §6 (`wait_bounded`,
//! `waits_end`) and the two-room reply and timeout, through the tool and
//! the desk's safe-point door.

use std::sync::{Arc, Mutex};

use kernel::event::record::{SignalKind, WaitEnd};
use kernel::{Address, Payload, RunId, TimeMs, Tool, ToolCall, ToolName, Version};
use proptest::prelude::*;
use serde_json::{Value, json};

use super::*;
use crate::inbox::{Inbox, Mailslot, Signal};
use crate::signal_desk::{Post, RoomMail};
use crate::signal_tool::SignalTool;

type Posted = Arc<Mutex<Vec<SignalEffect>>>;

const START: u64 = 1_700_000_000_000;

struct Room {
    desk: Arc<Mutex<SignalDesk>>,
    tool: SignalTool,
    posted: Posted,
    slot: Mailslot,
}

fn room(addr: &str) -> Room {
    let posted = Posted::default();
    let slot = Mailslot::default();
    let into = Arc::clone(&posted);
    let desk = Arc::new(Mutex::new(SignalDesk::new(
        RunId::CITY,
        Address::parse(addr).unwrap(),
        addr.to_owned(),
        Address::parse("lab").unwrap(),
        TimeMs::new(START),
        RoomMail {
            inbox: Inbox::new(64, 4),
            slot: slot.clone(),
            post: Post::new(move |line| {
                into.lock().unwrap().push(line.clone());
                Ok(())
            }),
        },
    )));
    let tool = SignalTool::new(Arc::clone(&desk)).unwrap();
    Room {
        desk,
        tool,
        posted,
        slot,
    }
}

impl Room {
    fn send(&self, to: &str, text: &str, wait: bool) -> Value {
        let args = json!({ "action": "send", "to": to, "text": text, "wait": wait });
        let outcome = self
            .tool
            .invoke(&ToolCall {
                id: "tu_1".to_owned(),
                name: ToolName::parse("signal").unwrap(),
                args: Payload::new(args.as_object().unwrap().clone()).unwrap(),
            })
            .unwrap();
        Value::Object(outcome.result.as_map().clone())
    }

    fn turn(&self, now: u64) -> WaitTurn {
        self.desk
            .lock()
            .unwrap()
            .wait_out(TimeMs::new(now))
            .unwrap()
    }

    fn waits(&self) -> Vec<SignalEffect> {
        self.posted
            .lock()
            .unwrap()
            .iter()
            .filter(|line| {
                matches!(
                    line,
                    SignalEffect::WaitStarted(_) | SignalEffect::WaitEnded(_)
                )
            })
            .cloned()
            .collect()
    }

    /// The signals this room has posted for `to`, as the city would
    /// drop them into the receiving run's slot.
    fn deliver_to(&self, to: &Room) {
        for line in self.posted.lock().unwrap().drain(..) {
            if let SignalEffect::Enqueued(signal) = line {
                to.slot.drop_in(signal, 64).unwrap();
            }
        }
    }
}

/// Who replied and what they said, from a turn that must be a reply.
fn replied(turn: WaitTurn) -> (String, String) {
    let WaitTurn::Replied(letter) = turn else {
        panic!("the wait ended on a reply, got {turn:?}");
    };
    assert_eq!(letter.kind(), crate::LetterKind::Reply);
    (letter.from().to_owned(), letter.text().to_owned())
}

fn letter(from: &str, to: &str, n: u32) -> Signal {
    let mut payload = serde_json::Map::new();
    payload.insert("text".to_owned(), Value::String(format!("letter {n}")));
    Signal::new(
        kernel::event::record::SignalId::parse(&format!("other-s{n}")).unwrap(),
        SignalKind::Mention,
        from.to_owned(),
        Address::parse(to).unwrap(),
        Version::FIRST,
        Payload::new(payload).unwrap(),
        TimeMs::new(START),
    )
    .unwrap()
}

/// A sends to B with `wait`, and B replies before A reaches the stop: a
/// later safe point of A's wave empties the slot first (`take_steer`).
/// The reply is kept for the wait rather than queued, so the stop ends on
/// it instead of running to the deadline (collab D15, `spec/Delivery.lean`
/// §8 `kept_is_first_reply`), and a letter from a third room still queues.
#[test]
fn a_reply_that_lands_before_the_stop_still_ends_the_wait() {
    let (a, b) = (room("lab/a"), room("lab/b"));
    a.send("lab/b", "is the kiln free?", true);
    b.send("lab/a", "free now", false);
    b.deliver_to(&a);
    a.slot.drop_in(letter("lab/c", "lab/a", 1), 64).unwrap();
    assert_eq!(a.desk.lock().unwrap().take_steer().unwrap(), None);
    assert_eq!(
        replied(a.turn(START + 1)),
        ("lab/b".to_owned(), "free now".to_owned())
    );
    assert_eq!(
        a.desk.lock().unwrap().pending(),
        1,
        "the third room's letter queues"
    );
}

/// A sends to B with `wait`; B replies; A's next safe point ends the
/// wait with B's words, and the ledger pairs the start with a reply end.
#[test]
fn a_waiting_send_resumes_with_the_reply() {
    let (a, b) = (room("lab/a"), room("lab/b"));
    let sent = a.send("lab/b", "is the kiln free?", true);
    assert!(sent["waiting"].as_str().unwrap().contains("lab/b replies"));
    assert_eq!(a.turn(START + 1), WaitTurn::Waiting);
    b.send("lab/a", "free now", false);
    b.deliver_to(&a);
    assert_eq!(
        replied(a.turn(START + 2)),
        ("lab/b".to_owned(), "free now".to_owned())
    );
    assert_eq!(a.turn(START + 3), WaitTurn::Idle);
    let ends: Vec<_> = a
        .waits()
        .into_iter()
        .filter_map(|line| match line {
            SignalEffect::WaitEnded(end) => Some(end.by),
            _ => None,
        })
        .collect();
    assert!(matches!(ends.as_slice(), [WaitEnd::Reply { .. }]));
}

/// Nobody replies: every safe point before the deadline answers
/// `Waiting`, so the lane makes no model call, and the one at the
/// deadline ends the wait with the timeout text.
#[test]
fn an_unanswered_wait_ends_at_the_deadline() {
    let a = room("lab/a");
    a.send("lab/b", "anyone?", true);
    for now in [START, START + PATIENCE_MS / 2, START + PATIENCE_MS - 1] {
        assert_eq!(a.turn(now), WaitTurn::Waiting);
    }
    assert_eq!(
        a.turn(START + PATIENCE_MS),
        WaitTurn::TimedOut {
            text: format!(
                "no reply came from lab/b within {} s; go on without it",
                PATIENCE_MS / 1000
            ),
        }
    );
    assert!(matches!(
        a.waits().last(),
        Some(SignalEffect::WaitEnded(end)) if end.by == WaitEnd::Timeout
    ));
}

/// One step of `Collab.Delivery.Act` as one waiting room sees it: a
/// send (with or without `wait`) to `lab/b`, a reply from `lab/b`, a
/// letter from someone else, a tick of the injected clock, a leave.
#[derive(Debug, Clone)]
enum Act {
    Send { wait: bool },
    Reply,
    Other,
    Tick { ms: u64 },
    Leave,
}

fn act() -> impl Strategy<Value = Act> {
    prop_oneof![
        any::<bool>().prop_map(|wait| Act::Send { wait }),
        Just(Act::Reply),
        Just(Act::Other),
        (1..=PATIENCE_MS / 3).prop_map(|ms| Act::Tick { ms }),
        Just(Act::Leave),
    ]
}

/// Steps one trace and checks after every safe point: `wait_bounded`
/// (every started deadline is at most `PATIENCE_MS` after the reading
/// it started at) and `waits_end` (no wait is still open once the clock
/// passed `PATIENCE_MS` since it started).
fn run_trace(acts: &[Act]) {
    let a = room("lab/a");
    let mut now = START;
    let mut letters = 0u32;
    let mut open_since: Option<u64> = None;
    for act in acts {
        match act {
            Act::Send { wait } => {
                let already = a.desk.lock().unwrap().waiting.is_some();
                if *wait && already {
                    continue;
                }
                a.send("lab/b", "hello", *wait);
                if *wait {
                    open_since = Some(now);
                }
            }
            Act::Reply => {
                letters += 1;
                a.slot
                    .drop_in(letter("lab/b", "lab/a", letters), 64)
                    .unwrap();
            }
            Act::Other => {
                letters += 1;
                a.slot
                    .drop_in(letter("lab/c", "lab/a", letters), 64)
                    .unwrap();
            }
            Act::Tick { ms } => now += ms,
            Act::Leave => {
                a.desk.lock().unwrap().wait_left().unwrap();
                open_since = None;
            }
        }
        let before = a.waits().len();
        let turn = a.turn(now);
        if let Some(SignalEffect::WaitStarted(started)) = a.waits().get(before) {
            let deadline = started.deadline_ms;
            assert!(
                deadline <= now + PATIENCE_MS,
                "wait_bounded: {deadline} > {now} + patience"
            );
        }
        match turn {
            WaitTurn::Replied(_) | WaitTurn::TimedOut { .. } | WaitTurn::Idle => open_since = None,
            WaitTurn::Waiting => {
                let since = open_since.unwrap();
                assert!(
                    now < since + PATIENCE_MS,
                    "waits_end: a wait open at {now}, started {since}"
                );
            }
        }
    }
}

proptest! {
    #[test]
    fn every_wait_is_bounded_and_ends(acts in proptest::collection::vec(act(), 0..40)) {
        run_trace(&acts);
    }
}
