// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use kernel::ActionFingerprint;
use kernel::stall::observe;

const BASES: [u64; 9] = [
    500, 1_000, 2_000, 4_000, 8_000, 16_000, 32_000, 60_000, 60_000,
];

fn run(tail: u8) -> RunId {
    let mut bytes = [
        0x01, 0x93, 0x2a, 0x7c, 0x10, 0x00, 0x70, 0x00, 0x80, 0, 0, 0, 0, 0, 0, 0,
    ];
    bytes[15] = tail;
    RunId::from_bytes(bytes)
}

fn waits(dog: &mut Watchdog, count: usize) -> Vec<u64> {
    (0..count)
        .map(
            |_| match dog.on_provider_failure(&provider_error(true), TimeMs::new(0)) {
                Disposal::BackOff { until, .. } => until.value(),
                other => panic!("a retriable failure under its ceiling backs off: {other:?}"),
            },
        )
        .collect()
}

fn within_schedule(waits: &[u64]) -> bool {
    waits
        .iter()
        .zip(BASES)
        .all(|(wait, base)| (base..=base + base / 2).contains(wait))
}

#[test]
fn each_run_waits_its_own_jittered_schedule() {
    let schedule = |tail: u8| waits(&mut Watchdog::new(Retries::UntilHalted, run(tail)), 9);
    let (first, second) = (schedule(1), schedule(2));
    assert!(
        within_schedule(&first) && within_schedule(&second),
        "{first:?} {second:?}"
    );
    assert_eq!(first, schedule(1), "the same run waits the same schedule");
    assert_ne!(first, second, "two runs do not ask again in step");
}

#[test]
fn disposal_is_graded_steer_first_freeze_second() {
    let mut dog = Watchdog::new(Retries::UntilHalted, run(1));
    let same = ActionFingerprint::derive(b"exec identical");
    let sample = vec![same, same, same];
    let verdict = observe(&sample);
    let first = dog.on_stall(&verdict);
    match &first {
        Disposal::CorrectiveSteer { text } => {
            assert!(text.contains("repeated the same call 3 times"))
        }
        other => panic!("first hit must steer, got {other:?}"),
    }
    let second = dog.on_stall(&verdict);
    assert_eq!(
        second,
        Disposal::Freeze {
            reason: FreezeReason::Stall
        }
    );

    let payload = serde_json::to_value(dog.fired_payload(&second).unwrap()).unwrap();
    assert_eq!(payload["action"], "freeze");
    assert_eq!(payload["reason"], "stall");

    assert_eq!(
        dog.fired_payload(&Disposal::Proceed)
            .unwrap_err()
            .recovery(),
        "report this against runtime::watchdog: `Proceed` is the verdict that records nothing, and the caller asked it for a payload"
    );
}

#[test]
fn ok_verdicts_never_dispose() {
    let mut dog = Watchdog::new(Retries::UntilHalted, run(1));
    assert_eq!(dog.on_stall(&StallVerdict::Ok), Disposal::Proceed);
    assert_eq!(dog.on_stall(&StallVerdict::Ok), Disposal::Proceed);
}

fn provider_error(retriable: bool) -> AxError {
    let draft = AxError::failure(AxCode::Provider, "call the model", "the provider said no");
    let draft = if retriable { draft.retriable() } else { draft };
    draft.with_recovery("dispatch again, or attach a second endpoint")
}

#[test]
fn a_failure_the_provider_will_repeat_stops_after_one() {
    let mut dog = Watchdog::new(Retries::UntilHalted, run(1));
    assert_eq!(
        dog.on_provider_failure(&provider_error(false), TimeMs::new(9_000)),
        Disposal::Freeze {
            reason: FreezeReason::ProviderRefused
        },
        "a request the provider rejected on its shape buys the same rejection again"
    );
}

#[test]
fn an_answer_ends_the_streak_the_backoff_counts() {
    let mut dog = Watchdog::new(Retries::AtMost(2), run(1));
    let before = waits(&mut dog, 2);
    assert!(within_schedule(&before), "{before:?}");
    dog.on_provider_answered();
    assert_eq!(
        waits(&mut dog, 2),
        before,
        "an outage after a recovery starts from the first wait, under a fresh ceiling"
    );
}

#[test]
fn a_retriable_failure_backs_off_and_never_freezes_by_itself() {
    let mut dog = Watchdog::new(Retries::UntilHalted, run(1));
    let waits: Vec<u64> = (0..64u64)
        .map(
            |_| match dog.on_provider_failure(&provider_error(true), TimeMs::new(1_000)) {
                Disposal::BackOff { until, .. } => until.value().saturating_sub(1_000),
                other => {
                    panic!("only Halt stops a run that is waiting out a provider: {other:?}")
                }
            },
        )
        .collect();
    assert!(
        within_schedule(&waits),
        "each failure in a row doubles the wait, up to a minute: {waits:?}"
    );
    assert!(
        waits[9..]
            .iter()
            .all(|wait| (60_000..=90_000).contains(wait))
    );
    let payload = serde_json::to_value(
        dog.fired_payload(&Disposal::BackOff {
            until: TimeMs::new(1_000),
            code: AxCode::Provider,
            subject: "the provider said no".to_owned(),
        })
        .unwrap(),
    )
    .unwrap();
    assert_eq!(payload["action"], "back_off");
    assert_eq!(payload["until_ms"], 1_000);
    assert_eq!(payload["provider_failures"], 64);
    assert_eq!(
        payload["subject"], "the provider said no",
        "a line saying it backed off says what it backed off from"
    );
}

fn waits_of_fresh_run() -> Vec<u64> {
    waits(&mut Watchdog::new(Retries::UntilHalted, run(1)), 2)
}

use kernel::ProviderFailureKind;
use kernel::account_recovery::AccountRetries;
use kernel::model::{AccountRoster, RosterEntry};

fn label(name: &str) -> ServerLabel {
    ServerLabel::parse(name).unwrap()
}

fn roster(accounts: &[(&str, bool)], current: &str, retries: AccountRetries) -> AccountRoster {
    AccountRoster {
        provider: "house".to_owned(),
        accounts: accounts
            .iter()
            .map(|(name, usable)| RosterEntry {
                account: label(name),
                usable: *usable,
            })
            .collect(),
        current: label(current),
        retries,
    }
}

fn rejected_key() -> AxError {
    AxError::provider(
        ProviderFailureKind::Refused { status: 401 },
        "call the model",
        "401 Unauthorized",
    )
    .account_unusable()
    .with_recovery("file another key")
}

fn busy() -> AxError {
    AxError::provider(
        ProviderFailureKind::Refused { status: 429 },
        "call the model",
        "429 Too Many Requests",
    )
    .retriable()
    .with_recovery("wait, then ask again")
}

fn lost() -> AxError {
    AxError::provider(ProviderFailureKind::Cut, "call the model", "stream cut")
        .effect_unknown()
        .with_recovery("ask again")
}

fn switched_to(name: &str, failure: &AxError) -> Disposal {
    Disposal::Switch {
        to: label(name),
        code: *failure.code(),
        subject: failure.subject().to_owned(),
    }
}

#[test]
fn a_round_opens_on_the_current_account_or_the_first_redeemable_one() {
    let mut dog = Watchdog::new(Retries::UntilHalted, run(1));
    let opened = [
        roster(&[("a", true), ("b", true)], "b", AccountRetries::Two),
        roster(&[("a", false), ("b", true)], "a", AccountRetries::Two),
    ]
    .map(|listed| {
        let account = dog.open_round(Some(listed)).unwrap();
        dog.on_provider_answered();
        account
    });
    assert_eq!(opened, [Some(label("b")), Some(label("b"))]);
    assert_eq!(
        dog.open_round(Some(roster(&[("solo", true)], "solo", AccountRetries::Two)))
            .unwrap(),
        None,
        "one account names nothing to select"
    );
}

#[test]
fn several_accounts_switch_on_a_rejected_key_and_after_the_busy_budget() {
    let mut dog = Watchdog::new(Retries::UntilHalted, run(1));
    dog.open_round(Some(roster(
        &[("a", true), ("b", true), ("c", true)],
        "a",
        AccountRetries::One,
    )))
    .unwrap();
    let first = dog.on_provider_failure(&rejected_key(), TimeMs::new(0));
    assert_eq!(first, switched_to("b", &rejected_key()));
    let payload = serde_json::to_value(dog.fired_payload(&first).unwrap()).unwrap();
    assert_eq!(
        (&payload["action"], &payload["to"], &payload["code"]),
        (
            &serde_json::json!("switch"),
            &serde_json::json!("b"),
            &serde_json::json!("E_PROVIDER")
        )
    );
    let waits = waits(&mut dog, 1);
    assert!(
        within_schedule(&waits),
        "the new account starts from the first wait: {waits:?}"
    );
    assert_eq!(
        dog.on_provider_failure(&busy(), TimeMs::new(0)),
        switched_to("c", &busy()),
        "one resend spent, the next account takes the request"
    );
}

#[test]
fn a_round_with_no_account_left_freezes_as_exhausted() {
    let mut dog = Watchdog::new(Retries::UntilHalted, run(1));
    dog.open_round(Some(roster(
        &[("a", true), ("b", true), ("c", false)],
        "a",
        AccountRetries::Two,
    )))
    .unwrap();
    assert_eq!(
        dog.on_provider_failure(&rejected_key(), TimeMs::new(0)),
        switched_to("b", &rejected_key())
    );
    assert_eq!(
        dog.on_provider_failure(&rejected_key(), TimeMs::new(0)),
        Disposal::Freeze {
            reason: FreezeReason::AccountsExhausted
        }
    );
    let exhausted = dog.exhausted();
    assert_eq!(
        (*exhausted.code(), exhausted.subject()),
        (
            AxCode::ProviderAccountsExhausted,
            "house: a E_PROVIDER Refused { status: 401 }; \
             b E_PROVIDER Refused { status: 401 }; c not redeemable"
        )
    );
    let none = Watchdog::new(Retries::UntilHalted, run(1)).open_round(Some(roster(
        &[("a", false), ("b", false)],
        "a",
        AccountRetries::Two,
    )));
    assert_eq!(
        none.map_err(|err| (*err.code(), err.subject().to_owned())),
        Err((
            AxCode::ProviderAccountsExhausted,
            "house: a not redeemable; b not redeemable".to_owned()
        ))
    );
}

#[test]
fn an_unknown_effect_never_switches_the_account() {
    let mut dog = Watchdog::new(Retries::UntilHalted, run(1));
    dog.open_round(Some(roster(
        &[("a", true), ("b", true)],
        "a",
        AccountRetries::One,
    )))
    .unwrap();
    assert!(matches!(
        dog.on_provider_failure(&lost(), TimeMs::new(0)),
        Disposal::BackOff { .. }
    ));
    assert_eq!(
        dog.on_provider_failure(&lost(), TimeMs::new(0)),
        Disposal::Freeze {
            reason: FreezeReason::ProviderRefused
        }
    );
}

#[test]
fn a_repair_is_admitted_within_the_account_budget_and_counted() {
    let mut dog = Watchdog::new(Retries::UntilHalted, run(1));
    assert!(dog.admits_repair(), "no round: the repair goes out");
    dog.open_round(Some(roster(
        &[("a", true), ("b", true)],
        "a",
        AccountRetries::One,
    )))
    .unwrap();
    assert!(dog.admits_repair());
    dog.repaired();
    assert!(!dog.admits_repair(), "the account's one resend is spent");
    assert_eq!(
        dog.on_provider_failure(&busy(), TimeMs::new(0)),
        switched_to("b", &busy())
    );
    let mut single = Watchdog::new(Retries::AtMost(0), run(1));
    single.open_round(None).unwrap();
    single.repaired();
    assert!(single.admits_repair());
}

#[test]
fn a_provider_that_names_its_wait_is_not_asked_sooner() {
    let mut dog = Watchdog::new(Retries::UntilHalted, run(1));
    let told = |wait_ms: u64| {
        AxError::failure(AxCode::Provider, "call the model", "answered 429")
            .retriable_after(wait_ms)
            .with_recovery("wait, then ask again")
    };
    let waits: Vec<u64> = [30_000, 10]
        .map(
            |wait_ms| match dog.on_provider_failure(&told(wait_ms), TimeMs::new(1_000)) {
                Disposal::BackOff { until, .. } => until.value().saturating_sub(1_000),
                other => {
                    panic!("a provider that says when to come back is asked again: {other:?}")
                }
            },
        )
        .to_vec();
    let own = waits_of_fresh_run();
    assert_eq!(
        waits,
        [30_000, own[1]],
        "the longer of the provider's word and the schedule's own wait"
    );
}
