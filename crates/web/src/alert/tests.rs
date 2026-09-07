// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

//! One fact, one interruption, through the production door.

use channels::{AxError, EventKind, EventRecord};

use super::judge::{Alert, AlertKind, Alerts, Raise, absorb, refused};
use crate::lang::{Lang, say};

fn alert(kind: AlertKind, key: &str) -> Alert {
    Alert {
        kind,
        key: key.to_owned(),
        message: format!("{} at {key}", say(Lang::En, kind.title())),
    }
}

#[test]
fn one_fact_interrupts_once_however_often_it_is_seen() {
    let mut alerts = Alerts::new();
    let frozen = alert(AlertKind::RunFrozen, "run/7");
    assert_eq!(alerts.raise(&frozen), Raise::Interrupt);
    for _ in 0..100 {
        assert_eq!(alerts.raise(&frozen), Raise::Silent);
    }
}

#[test]
fn a_fact_that_ends_and_returns_is_a_new_fact() {
    let mut alerts = Alerts::new();
    let waiting = alert(AlertKind::AwaitingApproval, "ap/1");
    assert_eq!(alerts.raise(&waiting), Raise::Interrupt);
    assert!(alerts.clear("ap/1"));
    assert_eq!(
        alerts.raise(&waiting),
        Raise::Interrupt,
        "the second time is a real second time"
    );
}

#[test]
fn only_the_two_kinds_that_need_somebody_now_interrupt() {
    // Teaching people to dismiss notifications costs the two that
    // matter, so a refusal - already visible where they are working -
    // and a degraded provider only mark.
    let mut alerts = Alerts::new();
    assert_eq!(
        alerts.raise(&alert(AlertKind::Refused, "gate/1")),
        Raise::Mark
    );
    assert_eq!(
        alerts.raise(&alert(AlertKind::ProviderTrouble, "prov/1")),
        Raise::Mark
    );
    assert_eq!(
        alerts.raise(&alert(AlertKind::AwaitingApproval, "ap/2")),
        Raise::Interrupt
    );
    assert_eq!(
        alerts.raise(&alert(AlertKind::RunFrozen, "run/2")),
        Raise::Interrupt
    );
}

fn recorded(kind: EventKind, data: serde_json::Map<String, serde_json::Value>) -> EventRecord {
    EventRecord::from_draft(
        channels::EventDraft {
            run: channels::RunId::from_bytes([4u8; 16]),
            t: channels::TimeMs::new(1),
            who: "gate".to_owned(),
            addr: None,
            kind,
            data: channels::Payload::new(data).unwrap(),
            ig: false,
        },
        channels::Seq::new(1),
        channels::B3Hash::digest(b"prev"),
    )
}

fn with_id(id: &str) -> serde_json::Map<String, serde_json::Value> {
    let mut map = serde_json::Map::new();
    map.insert("id".to_owned(), serde_json::Value::String(id.to_owned()));
    map.insert(
        "action_desc".to_owned(),
        serde_json::Value::String("push to the remote".to_owned()),
    );
    map
}

#[test]
fn an_answered_question_stops_asking_and_a_new_one_asks_again() {
    let mut alerts = Alerts::new();
    let asked = recorded(EventKind::ApprovalRequested, with_id("item-7"));
    assert_eq!(absorb(Lang::En, &mut alerts, &asked), Raise::Interrupt);
    // A reconnect re-delivers what it already sent; one fact, one
    // interruption.
    assert_eq!(absorb(Lang::En, &mut alerts, &asked), Raise::Silent);

    let answered = recorded(EventKind::ApprovalResolved, with_id("item-7"));
    assert_eq!(absorb(Lang::En, &mut alerts, &answered), Raise::Silent);
    assert_eq!(
        absorb(Lang::En, &mut alerts, &asked),
        Raise::Interrupt,
        "asked again after being answered is a new fact"
    );
}

#[test]
fn a_sick_provider_is_one_fact_however_many_calls_notice_it() {
    let mut alerts = Alerts::new();
    let degraded = recorded(EventKind::ProviderDegraded, serde_json::Map::new());
    let lost = recorded(EventKind::EndpointLost, serde_json::Map::new());
    assert_eq!(absorb(Lang::En, &mut alerts, &degraded), Raise::Mark);
    assert_eq!(absorb(Lang::En, &mut alerts, &lost), Raise::Silent);
    assert_eq!(
        absorb(
            Lang::En,
            &mut alerts,
            &recorded(EventKind::EndpointAttached, serde_json::Map::new())
        ),
        Raise::Silent
    );
    assert_eq!(
        absorb(Lang::En, &mut alerts, &degraded),
        Raise::Mark,
        "a provider that goes bad again is a second fact"
    );
}

#[test]
fn most_of_what_happens_needs_nobody() {
    let mut alerts = Alerts::new();
    for kind in [
        EventKind::ToolCalled,
        EventKind::ToolResult,
        EventKind::ModelReturned,
        EventKind::RunStarted,
        EventKind::SignalEnqueued,
    ] {
        assert_eq!(
            absorb(
                Lang::En,
                &mut alerts,
                &recorded(kind, serde_json::Map::new())
            ),
            Raise::Silent,
            "{kind:?} interrupted somebody"
        );
    }
}

#[test]
fn clearing_something_never_raised_is_not_an_error() {
    let mut alerts = Alerts::new();
    assert!(!alerts.clear("nothing"));
}

#[test]
fn a_refusal_reaches_the_person_with_its_way_out_attached() {
    let told = refused(
        Lang::En,
        &AxError::failure(
            channels::AxCode::ConfigInvalid,
            "attach an endpoint",
            "modelscope",
        )
        .with_recovery("the base url needs its /v1"),
    );
    assert_eq!(told.code, "E_CONFIG_INVALID");
    assert_eq!(told.what, "cannot attach an endpoint on modelscope");
    assert_eq!(told.recovery, "the base url needs its /v1");
}

/// A refusal with an empty recovery says so rather than rendering a
/// blank line, because a blank line reads as "nothing is wrong".
#[test]
fn a_refusal_with_no_way_out_says_that_much() {
    let told = refused(
        Lang::En,
        &AxError::failure(
            channels::AxCode::StorageFatal,
            "append to the ledger",
            "the disk is full",
        ),
    );
    assert!(!told.recovery.is_empty());
}

/// Two identical refusals are two answers. The dedup that protects a
/// person from a run frozen for an hour would, applied here, swallow
/// the answer to their second attempt.
#[test]
fn the_same_refusal_twice_is_two_answers_not_one_fact() {
    let err = AxError::failure(channels::AxCode::InvalidArgs, "select a model", "nowhere");
    assert_eq!(refused(Lang::En, &err), refused(Lang::En, &err));
}
