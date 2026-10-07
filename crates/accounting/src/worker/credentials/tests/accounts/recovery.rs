// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How one model call's account round meets each kind of failure on
//! the production path (`crates/kernel/Spec.lean` §8-86, runtime D89):
//! resend on the same account, switch to the next, or stop.

use super::*;

/// A busy account is asked again within its budget (two resends by
/// default), then the next account takes the request at once; the
/// Session then keeps the account that answered, for later runs and
/// after the first account is moved back to the front, though that
/// account now answers too.
#[test]
fn a_busy_account_is_asked_its_budget_then_the_next_keeps_the_session() {
    let mut rig = Rig::new(
        vec![busy(), busy(), busy(), answer()],
        vec![answer()],
        unpaced(),
    );
    rig.dispatch();
    assert_eq!(rig.wire_accounts(), ["a", "a", "a", "b"]);
    assert_eq!(rig.watchdog(), ["back_off", "back_off", "switch b"]);
    assert_eq!(rig.bound().as_deref(), Some("b"));

    rig.dispatch();
    rig.attach(&["b", "a"], None);
    rig.attach(&["a", "b"], None);
    rig.dispatch();
    assert_eq!(
        rig.wire_accounts(),
        ["a", "a", "a", "b", "b", "b"],
        "a reorder moves no healthy Session"
    );
    assert_eq!(rig.bound().as_deref(), Some("b"));
    rig.assert_keys_stay_in_the_vault();
}

/// A key the provider rejects and a quota that is used up both move the
/// request to the next account after one send, with no wait.
#[test]
fn a_rejected_key_or_a_spent_quota_moves_on_after_one_send() {
    for refusal in [rejected(), spent()] {
        let mut rig = Rig::new(vec![refusal.clone()], vec![answer()], unpaced());
        rig.dispatch();
        assert_eq!(rig.wire_accounts(), ["a", "b"], "{refusal:?}");
        assert_eq!(rig.watchdog(), ["switch b"], "{refusal:?}");
        assert_eq!(rig.bound().as_deref(), Some("b"));
        rig.assert_keys_stay_in_the_vault();
    }
}

/// A request the provider rejects on its shape would be rejected on any
/// account: it is sent once, and the run stops with the refusal.
#[test]
fn a_malformed_request_is_sent_once_and_never_moves() {
    let mut rig = Rig::new(
        vec![Reply::refused(
            400,
            &serde_json::json!({ "code": "invalid_request_error" }),
        )],
        vec![answer()],
        unpaced(),
    );
    rig.dispatch();
    assert_eq!(rig.wire_accounts(), ["a"]);
    assert!(
        rig.watchdog().is_empty(),
        "nothing was waited out or switched"
    );
    let refused = rig.records("provider_degraded");
    assert_eq!(
        refused
            .iter()
            .map(|line| (&line["data"]["code"], &line["data"]["provider"]))
            .collect::<Vec<_>>(),
        [(
            &serde_json::json!("E_PROVIDER"),
            &serde_json::json!({ "kind": "refused", "status": 400 })
        )]
    );
    assert_eq!(rig.bound(), None);
    rig.assert_keys_stay_in_the_vault();
}

/// An answer lost after the request went out may have been billed, so it
/// is asked again on the same account only, within that account's
/// budget, and the run stops with the effect still unknown; the budget
/// is the one the person chose on the advanced form.
#[test]
fn a_lost_answer_is_asked_again_on_its_own_account_only() {
    for (retries, sends) in [(None, 3), (Some(AccountRetries::One), 2)] {
        let mut rig = Rig::new(vec![Reply::Lost], vec![answer()], unpaced());
        rig.attach(&["a", "b"], retries);
        rig.dispatch();
        assert_eq!(rig.wire_accounts(), vec!["a"; sends], "{retries:?}");
        assert_eq!(rig.watchdog(), vec!["back_off"; sends - 1]);
        let lost = rig.records("provider_degraded");
        assert_eq!(
            lost.iter()
                .map(|line| &line["data"]["retry"])
                .collect::<Vec<_>>(),
            [&serde_json::json!("unknown")],
            "the run stops saying the effect is unknown"
        );
        assert_eq!(rig.bound(), None);
        rig.assert_keys_stay_in_the_vault();
    }
}

/// Every account failing in one round leaves one refusal of its own,
/// which names each account and what it met, and neither key nor
/// reference; no account's own failure is recorded as the run's end.
#[test]
fn every_account_failing_records_one_exhausted_refusal() {
    let mut rig = Rig::new(vec![rejected()], vec![spent()], unpaced());
    rig.dispatch();
    assert_eq!(rig.wire_accounts(), ["a", "b"]);
    assert_eq!(rig.watchdog(), ["switch b"]);
    let ends = rig
        .records("provider_degraded")
        .iter()
        .map(|line| {
            (
                line["data"]["code"].as_str().unwrap_or_default().to_owned(),
                line["data"]["subject"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        ends,
        [(
            "E_PROVIDER_ACCOUNTS_EXHAUSTED".to_owned(),
            "house: a E_PROVIDER Refused { status: 401 }; b E_PROVIDER Quota { status: 429 }"
                .to_owned()
        )]
    );
    assert!(
        rig.records("provider_degraded")
            .iter()
            .all(|line| !line.to_string().contains("secret:")),
        "the refusal names no reference"
    );
    assert_eq!(rig.bound(), None);
    rig.assert_keys_stay_in_the_vault();
}

/// A stop reaches a run while it waits out a busy account: the provider
/// asked for a minute, and the run ends inside that wait, cancelled,
/// with no second request on either account. The run here is
/// a root run, which the person stops with `Cancel`; a handed-down run is
/// reached by `Halt` through its backlog member, and the wait asks both
/// in one place (`Interrupting::halted`).
#[test]
fn a_stop_reaches_a_run_waiting_out_a_busy_account() {
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    let refused = std::sync::Arc::new(AtomicBool::new(false));
    let seen = std::sync::Arc::clone(&refused);
    let pace: Pace = std::sync::Arc::new(move |request: &str| {
        if request.starts_with("POST ") {
            seen.store(true, Ordering::SeqCst);
        }
    });
    let mut rig = Rig::new(
        vec![busy().with_header("retry-after", "60")],
        vec![answer()],
        pace,
    );
    let asked = std::sync::Arc::new(AtomicU32::new(0));
    let asking = std::sync::Arc::clone(&asked);
    // The first question after the refusal is the wait's own, because a
    // run has no safe point between a failed call and its wait. That one
    // is let through, so the wait has slept a slice when the stop lands.
    rig.worker.serve(crate::worker::fixture::only_interrupts(
        std::sync::Arc::new(move |_run| {
            if !refused.load(Ordering::SeqCst) {
                return runtime::Interrupt::None;
            }
            match asking.fetch_add(1, Ordering::SeqCst) {
                0 => runtime::Interrupt::None,
                _ => runtime::Interrupt::Cancel,
            }
        }),
    ));
    rig.dispatch();
    assert!(
        asked.load(Ordering::SeqCst) >= 2,
        "the wait was never asked twice"
    );
    // Had the stop not landed, the run would have waited the minute out
    // and asked the busy account again: one request is the proof.
    assert_eq!(rig.wire_accounts(), ["a"]);
    assert_eq!(rig.watchdog(), ["back_off"]);
    let frozen = rig.records("run_frozen");
    assert_eq!(
        frozen
            .iter()
            .map(|line| &line["data"]["completion"])
            .collect::<Vec<_>>(),
        [&serde_json::json!("cancelled")]
    );
    rig.assert_keys_stay_in_the_vault();
}
