// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "test code: this driver names the outcomes it asserts on               and treats the rest as one arm, rather than modelling an               enum the assertion does not reach"
)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::redundant_closure,
    clippy::disallowed_methods,
    reason = "test code"
)]

use kernel::ledger::chain_hash;
use kernel::model::message_payload;
use kernel::{
    Address, AxError, B3Hash, BuildingPolicy, Completion, ContentBlock, EventDraft, EventRef,
    GENESIS_PREV, Ledger, Locator, Model, ModelRequest, ModelReturn, Payload, RunId, TimeMs,
    ToolCall, ToolName, ToolOutcome,
};
use runtime::handoff::Handoff;
use runtime::prefix::{FrozenPrefix, FrozenSegment, SegmentSlot};
use runtime::run::{Advance, Run, RunHooks, RunPlan, SafePoint, drive};
use runtime::turn::{CallShape, Interrupt};

use super::concurrency::{READING, SlowRead, WRITING, one_read_while_generating};
use super::meeting::{OneByOne, Placed, read_meta};
use super::support::*;

/// A read the model hands over before it finishes writing runs while it
/// writes: the turn takes about the longer of the two, not their sum,
/// and the ledger is the one the same read leaves when it runs after the
/// answer settles.
#[test]
fn a_read_handed_over_while_the_model_writes_runs_during_the_writing() {
    let (serial, _) = one_read_while_generating(&mut OneByOne(Placed::of(Box::new(SlowRead))));

    let (speculated, took) = one_read_while_generating(&mut Placed::of(Box::new(SlowRead)));

    assert!(
        took < WRITING + READING,
        "the read waited for the model to finish writing: the run took {took:?}"
    );
    assert_eq!(speculated.lines, serial.lines);
}

/// How long the model behind the keep-warm door keeps writing while it
/// waits for the read it handed over to start. A door that drops the
/// early call makes it wait all of this before it answers.
const READ_START_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

/// What happened behind the keep-warm door, in the order it happened.
type Happened = std::sync::Arc<std::sync::Mutex<Vec<&'static str>>>;

/// A read-only tool that notes when it starts and tells the model.
struct AnnouncedRead {
    happened: Happened,
    started: std::sync::mpsc::Sender<()>,
}

impl kernel::Tool for AnnouncedRead {
    fn meta(&self) -> &kernel::ToolMeta {
        static META: std::sync::OnceLock<kernel::ToolMeta> = std::sync::OnceLock::new();
        META.get_or_init(read_meta)
    }

    fn invoke(&self, _call: &ToolCall) -> Result<ToolOutcome, AxError> {
        self.happened.lock().unwrap().push("read started");
        self.started.send(()).unwrap();
        Ok(ToolOutcome {
            result: Payload::empty(),
            attachments: Vec::new(),
        })
    }
}

/// A streaming model that hands each call of its wave over as the call
/// completes, then keeps writing until the read it handed over has
/// started or [`READ_START_WAIT`] has passed.
struct WaitsForItsRead {
    waves: Vec<Vec<ToolCall>>,
    happened: Happened,
    started: std::sync::mpsc::Receiver<()>,
}

impl Model for WaitsForItsRead {
    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
        self.call_speculating(req, &mut |_: &kernel::Increment| {}, &mut |_: &ToolCall| {})
    }

    fn call_speculating(
        &mut self,
        _req: &ModelRequest,
        _onto: kernel::Increments<'_>,
        early: kernel::EarlyCalls<'_>,
    ) -> Result<ModelReturn, AxError> {
        let calls = if self.waves.is_empty() {
            Vec::new()
        } else {
            self.waves.remove(0)
        };
        calls.iter().for_each(|call| early(call));
        if !calls.is_empty() {
            // Timing out is not a failure here: the order in `happened`
            // is what the test reads.
            let _waited = self.started.recv_timeout(READ_START_WAIT);
        }
        self.happened.lock().unwrap().push("model returned");
        Ok(ModelReturn::bare(
            message_payload(&[ContentBlock::Text {
                text: "reading".to_owned(),
            }])
            .unwrap(),
            calls,
        ))
    }
}

/// Every run's model is wrapped in the keep-warm door, and the door hands
/// a read over as early as the model behind it does: the read starts
/// while the model is still writing, and the turn that follows answers
/// it from what the early start returned rather than starting it again.
#[test]
fn a_read_handed_over_behind_the_keep_warm_door_starts_before_the_model_returns() {
    let happened = Happened::default();
    let (started, heard) = std::sync::mpsc::channel();
    let model = WaitsForItsRead {
        waves: vec![vec![ToolCall {
            id: "r1".to_owned(),
            name: ToolName::parse("read").unwrap(),
            args: Payload::empty(),
        }]],
        happened: happened.clone(),
        started: heard,
    };
    let mut door = runtime::prefix::warmth::Warmed::new(
        Box::new(model),
        kernel::keep_warm::KeepWarm::default(),
        || Ok(TimeMs::new(0)),
        runtime::prefix::warmth::Lead::default(),
    );
    let mut invoke = Placed::of(Box::new(AnnouncedRead {
        happened: happened.clone(),
        started,
    }));
    let mut ledger = RecordingLedger::new();
    let mut now = counter();
    let mut interrupt = |_: SafePoint| Interrupt::None;
    let mut hooks = RunHooks {
        now: &mut now,
        monotonic_us: &mut || 0,
        interrupt: &mut interrupt,
        checkpoint: None,
        writes: &|_: &kernel::ToolCall| kernel::Writes::Domain,
        invoke: &mut invoke,
        wait: &mut |_: TimeMs| runtime::NextCall::Allowed,
        deltas: None,
    };

    drive(plan(), &mut ledger, &mut door, &mut hooks, &handoff()).unwrap();

    assert_eq!(
        *happened.lock().unwrap(),
        vec!["read started", "model returned", "model returned"]
    );
}

/// How one account of [`Accounted`] answers every request sent on it.
#[derive(Clone, Copy)]
enum Answers {
    /// It answers; the first answer of the run asks for one tool call,
    /// so the run takes a second turn.
    Yes,
    /// Its key is rejected: no resend helps, another account may.
    RejectedKey,
}

/// A provider with named accounts. Every request goes out on the account
/// selected last, and the account it went out on is noted, so a test
/// reads which account each call used off the ledger and off the model.
struct Accounted {
    accounts: Vec<(&'static str, bool, Answers)>,
    current: usize,
    answered: u32,
    sent: std::rc::Rc<std::cell::RefCell<Vec<&'static str>>>,
}

impl Accounted {
    fn new(accounts: Vec<(&'static str, bool, Answers)>) -> Accounted {
        Accounted {
            accounts,
            current: 0,
            answered: 0,
            sent: std::rc::Rc::default(),
        }
    }

    fn label(name: &str) -> kernel::ServerLabel {
        kernel::ServerLabel::parse(name).unwrap()
    }
}

impl Model for Accounted {
    fn provider_account(&self) -> Option<kernel::event::record::ProviderAccountBinding> {
        Some(kernel::event::record::ProviderAccountBinding {
            provider: "house".to_owned(),
            account: Accounted::label(self.accounts[self.current].0),
        })
    }

    fn account_roster(&self) -> Option<kernel::model::AccountRoster> {
        Some(kernel::model::AccountRoster {
            provider: "house".to_owned(),
            accounts: self
                .accounts
                .iter()
                .map(|(name, usable, _)| kernel::model::RosterEntry {
                    account: Accounted::label(name),
                    usable: *usable,
                })
                .collect(),
            current: Accounted::label(self.accounts[self.current].0),
            retries: kernel::account_recovery::AccountRetries::One,
        })
    }

    fn select_account(&mut self, account: &kernel::ServerLabel) {
        if let Some(position) = self
            .accounts
            .iter()
            .position(|(name, _, _)| *name == account.as_str())
        {
            self.current = position;
        }
    }

    fn call(&mut self, _req: &ModelRequest) -> Result<ModelReturn, AxError> {
        let (name, _, answers) = self.accounts[self.current];
        self.sent.borrow_mut().push(name);
        match answers {
            Answers::RejectedKey => Err(AxError::provider(
                kernel::ProviderFailureKind::Refused { status: 401 },
                "call the model",
                "401 Unauthorized",
            )
            .account_unusable()
            .with_recovery("file another key for this account")),
            Answers::Yes => {
                self.answered = self.answered.saturating_add(1);
                let calls = if self.answered == 1 {
                    vec![call("t-1")]
                } else {
                    Vec::new()
                };
                Ok(ModelReturn::bare(
                    message_payload(&[ContentBlock::Text {
                        text: "working".to_owned(),
                    }])
                    .unwrap(),
                    calls,
                ))
            }
        }
    }
}

/// Drives `model` to its end with hooks whose wait refuses to be asked:
/// a switch goes out at once, so a test that switches never waits.
fn drive_accounts(model: &mut Accounted) -> (RecordingLedger, Completion) {
    let mut ledger = RecordingLedger::new();
    let mut now = counter();
    let mut interrupt = |_: SafePoint| Interrupt::None;
    let mut invoke = |_: &ToolCall, _: TimeMs| {
        Ok(ToolOutcome {
            result: Payload::empty(),
            attachments: Vec::new(),
        })
    };
    let mut hooks = RunHooks {
        now: &mut now,
        monotonic_us: &mut || 0,
        interrupt: &mut interrupt,
        checkpoint: None,
        writes: &|_: &kernel::ToolCall| kernel::Writes::Domain,
        invoke: &mut invoke,
        wait: &mut |_: TimeMs| panic!("no call in these runs waits"),
        deltas: None,
    };
    let frozen = drive(plan(), &mut ledger, model, &mut hooks, &handoff()).unwrap();
    let completion = frozen.completion().clone();
    (ledger, completion)
}

/// The data of every line of `kind`, in ledger order.
fn data_of(ledger: &RecordingLedger, kind: &str) -> Vec<serde_json::Value> {
    ledger
        .lines
        .iter()
        .map(|line| serde_json::from_slice::<serde_json::Value>(line).unwrap())
        .filter(|line| line["kind"] == kind)
        .map(|line| line["data"].clone())
        .collect()
}

/// A rejected key moves the call to the next account at once, the line
/// that says so comes before the call it announces, and the next model
/// call of the run starts on the account that answered rather than going
/// back to the first.
#[test]
fn a_rejected_key_switches_the_account_at_once_and_the_run_keeps_the_one_that_answered() {
    let mut model = Accounted::new(vec![
        ("a", true, Answers::RejectedKey),
        ("b", true, Answers::Yes),
    ]);
    let (ledger, completion) = drive_accounts(&mut model);

    assert_eq!(*model.sent.borrow(), ["a", "b", "b"]);
    let called: Vec<serde_json::Value> = data_of(&ledger, "model_called")
        .into_iter()
        .map(|data| data["provider_account"]["account"].clone())
        .collect();
    assert_eq!(called, ["a", "b", "b"].map(serde_json::Value::from));
    let fired = data_of(&ledger, "watchdog_fired");
    assert_eq!(
        fired
            .iter()
            .map(|data| (
                data["action"].clone(),
                data["to"].clone(),
                data["code"].clone()
            ))
            .collect::<Vec<_>>(),
        [(
            serde_json::json!("switch"),
            serde_json::json!("b"),
            serde_json::json!("E_PROVIDER")
        )]
    );
    let kinds = ledger.kinds();
    let at = |wanted: &str| {
        kinds
            .iter()
            .enumerate()
            .filter(|(_, kind)| *kind == wanted)
            .map(|(index, _)| index)
            .collect::<Vec<_>>()
    };
    let (switch, called) = (at("watchdog_fired")[0], at("model_called"));
    assert!(
        called[0] < switch && switch < called[1],
        "the switch is written between the call it answers and the call it announces: {kinds:?}"
    );
    assert!(!matches!(completion, Completion::Cancelled));
}

/// Every account failing ends the run with one refusal of its own, which
/// names each account and what it met; no account is asked twice.
#[test]
fn every_account_failing_records_one_exhausted_refusal() {
    let mut model = Accounted::new(vec![
        ("a", true, Answers::RejectedKey),
        ("b", true, Answers::RejectedKey),
    ]);
    let (ledger, completion) = drive_accounts(&mut model);

    assert_eq!(*model.sent.borrow(), ["a", "b"]);
    let degraded = data_of(&ledger, "provider_degraded");
    assert_eq!(
        degraded
            .iter()
            .map(|data| (data["code"].clone(), data["subject"].clone()))
            .collect::<Vec<_>>(),
        [(
            serde_json::json!("E_PROVIDER_ACCOUNTS_EXHAUSTED"),
            serde_json::json!(
                "house: a E_PROVIDER Refused { status: 401 }; b E_PROVIDER Refused { status: 401 }"
            )
        )]
    );
    assert!(matches!(completion, Completion::Cancelled));
    assert_eq!(
        data_of(&ledger, "run_frozen")[0]["failure"]["code"],
        serde_json::json!("E_PROVIDER_ACCOUNTS_EXHAUSTED"),
        "the verdict names the reason the run stopped, not the last refusal"
    );
}

/// A roster with no account whose key can be redeemed sends nothing: the
/// run records why and stops before any model call.
#[test]
fn a_roster_with_nothing_redeemable_sends_nothing() {
    let mut model = Accounted::new(vec![("a", false, Answers::Yes), ("b", false, Answers::Yes)]);
    let (ledger, completion) = drive_accounts(&mut model);

    assert!(model.sent.borrow().is_empty());
    assert!(data_of(&ledger, "model_called").is_empty());
    assert_eq!(
        data_of(&ledger, "provider_degraded")
            .iter()
            .map(|data| data["subject"].clone())
            .collect::<Vec<_>>(),
        [serde_json::json!(
            "house: a not redeemable; b not redeemable"
        )]
    );
    assert!(matches!(completion, Completion::Cancelled));
}
