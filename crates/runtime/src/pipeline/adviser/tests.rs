// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

//! What the window adviser does to a packaged result: the answer rides
//! the same events the caller appends, the score moves the budget, and
//! a fallback leaves every figure exactly where the city put it.

use super::super::tests::no_offload;
use super::super::{PackContext, package};
use super::*;
use crate::offload::OffloadSite;
use memory::Cas;

fn window() -> Window {
    let mut window = Window::new();
    window.push_task_lines("do it", "it is done", crate::window::Opening::FromJob);
    window
}

/// An adviser that answers whatever the test scripted, so the port's
/// own contract — validation, fallback, recording — is what is under
/// test rather than a transport.
fn scripted(answer: Result<AdviserAnswer, AdviserFailure>) -> Adviser {
    Adviser::with(move |_ask, _window| answer.clone())
}

#[test]
fn an_answer_to_another_question_is_a_fallback() {
    let mut adviser = scripted(Ok(AdviserAnswer::Score { score_bp: 1 }));
    let consultation = adviser.consult(Ask::noul("toolu_01", "material"), &window(), 7);
    assert_eq!(
        consultation,
        Consultation::FellBack {
            ask: AdviserAsk::Noul,
            subject: "toolu_01".to_owned(),
            reason: AdviserFailure::Unreadable,
        }
    );
    assert!(consultation.answer().is_none());
}

#[test]
fn a_choice_outside_the_offered_options_is_a_fallback() {
    let mut adviser = scripted(Ok(AdviserAnswer::Choice {
        chosen: "something-else".to_owned(),
    }));
    let consultation = adviser.consult(
        Ask::choice("dispatch", vec!["sonnet".to_owned(), "opus".to_owned()]),
        &window(),
        3,
    );
    assert_eq!(consultation.answer(), None);
    let Consultation::FellBack { reason, .. } = consultation else {
        panic!("an off-list choice is not an answer");
    };
    assert_eq!(reason, AdviserFailure::Unreadable);
}

#[test]
fn a_missing_adviser_falls_back_as_unavailable() {
    let mut adviser = Adviser::none();
    let consultation = adviser.consult(Ask::score("toolu_02", "text"), &window(), 0);
    let Consultation::FellBack { reason, ask, .. } = consultation else {
        panic!("no adviser means no answer");
    };
    assert_eq!(reason, AdviserFailure::Unavailable);
    assert_eq!(ask, AdviserAsk::Score);
}

#[test]
fn a_valid_answer_is_kept_and_records_the_two_lines() {
    let mut adviser = scripted(Ok(AdviserAnswer::Noul {
        keep: false,
        confidence_bp: 8_750,
    }));
    let consultation = adviser.consult(Ask::noul("toolu_01", "material"), &window(), 41);
    assert_eq!(
        consultation.answer(),
        Some(&AdviserAnswer::Noul {
            keep: false,
            confidence_bp: 8_750
        })
    );
    let payloads = consultation.payloads().unwrap();
    assert_eq!(
        payloads.first().unwrap().as_map().get("ask"),
        Some(&serde_json::json!("noul"))
    );
    assert_eq!(
        payloads.get(1).unwrap().as_map().get("keep"),
        Some(&serde_json::json!(false))
    );
    assert_eq!(
        payloads.get(1).unwrap().as_map().get("elapsed_ms"),
        Some(&serde_json::json!(41))
    );
}

#[test]
fn a_fallback_records_the_reason() {
    let mut adviser = scripted(Err(AdviserFailure::Timeout));
    let consultation = adviser.consult(Ask::score("toolu_09", "text"), &window(), 900);
    let payloads = consultation.payloads().unwrap();
    assert_eq!(
        payloads.get(1).unwrap().as_map().get("reason"),
        Some(&serde_json::json!("timeout"))
    );
    assert_eq!(
        payloads.get(1).unwrap().as_map().get("subject"),
        Some(&serde_json::json!("toolu_09"))
    );
}

/// A density score spends less before the cut, and the consultation's
/// two lines ride the same events the caller appends. The same result
/// without an adviser fits its budget and stays whole, which is what
/// makes the fallback's worst case the old behaviour.
#[test]
fn a_density_score_scales_the_budget_and_records_the_two_lines() {
    let text = "The first sentence says what this result is about. ".repeat(40);
    let mut window = Window::new();
    window.push_task_lines("t", "g", crate::window::Opening::FromJob);
    let mut adviser = Adviser::with(|_ask, _window| {
        Ok(kernel::event::record::AdviserAnswer::Score { score_bp: 1_000 })
    });
    let consultation = adviser.consult(Ask::score("result", text.as_str()), &window, 12);
    let mut ctx = no_offload(4_096);
    ctx.adviser = Some(consultation);
    let out = package(text.as_bytes(), ctx).unwrap();
    assert!(
        out.content.len() < text.len() && out.content.contains("[truncated: "),
        "a low score shortens what would otherwise fit"
    );
    assert_eq!(out.events.len(), 2, "asked then answered");
    let answered = serde_json::to_value(&out.events[1]).unwrap();
    assert_eq!(answered["ask"], "score");
    assert_eq!(answered["score_bp"], 1_000);
    assert_eq!(answered["elapsed_ms"], 12);
    assert_eq!(
        package(text.as_bytes(), no_offload(4_096)).unwrap().content,
        text,
        "without an adviser the same result is kept whole"
    );
}

/// A consultation with no answer changes nothing, and says why on the
/// ledger. The fallback is the behaviour a city without advisers had.
#[test]
fn a_fallback_is_recorded_and_leaves_every_figure_alone() {
    let text = "The first sentence says what this result is about. ".repeat(40);
    let mut window = Window::new();
    window.push_task_lines("t", "g", crate::window::Opening::FromJob);
    let mut adviser = Adviser::none();
    let consultation = adviser.consult(Ask::score("result", text.as_str()), &window, 0);
    let mut ctx = no_offload(4_096);
    ctx.adviser = Some(consultation);
    let out = package(text.as_bytes(), ctx).unwrap();
    assert_eq!(out.content, text, "no answer means no adjustment");
    assert_eq!(out.events.len(), 2);
    let fell_back = serde_json::to_value(&out.events[1]).unwrap();
    assert_eq!(fell_back["reason"], "unavailable");
    assert_eq!(fell_back["subject"], "result");
}

/// The adviser may take out what the city would have had to store
/// anyway, and may not turn a result that fits into something offload
/// refuses. In both cases its answer is on the ledger.
#[test]
fn an_advisers_not_needed_acts_only_where_a_store_could_hold_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut cas = Cas::open(&dir.path().join("cas")).unwrap();
    let env = dir.path().join("env");
    std::fs::create_dir_all(&env).unwrap();
    let mut window = Window::new();
    window.push_task_lines("t", "g", crate::window::Opening::FromJob);
    let mut adviser = Adviser::with(|_ask, _window| {
        Ok(kernel::event::record::AdviserAnswer::Noul {
            keep: false,
            confidence_bp: 9_000,
        })
    });
    let big = "The first sentence says what this result is about. ".repeat(400);
    let consultation = adviser.consult(Ask::noul("result", big.as_str()), &window, 5);
    let out = package(
        big.as_bytes(),
        PackContext {
            cap_bytes: 1_024,
            stamp: None,
            net_notice: false,
            steer: None,
            reminder: None,
            offload: Some(OffloadSite {
                cas: &mut cas,
                environment: &env,
            }),
            sieve: None,
            adviser: Some(consultation),
        },
    )
    .unwrap();
    assert!(
        out.content.contains("[offloaded: total"),
        "a result too big for the window leaves with a way back: {}",
        out.content
    );
    let small = "{\"ok\":true}";
    let consultation = adviser.consult(Ask::noul("result", small), &window, 5);
    let out = package(
        small.as_bytes(),
        PackContext {
            cap_bytes: 4_096,
            stamp: None,
            net_notice: false,
            steer: None,
            reminder: None,
            offload: Some(OffloadSite {
                cas: &mut cas,
                environment: &env,
            }),
            sieve: None,
            adviser: Some(consultation),
        },
    )
    .unwrap();
    assert_eq!(out.content, small, "a result that fits is not turned away");
    assert_eq!(out.events.len(), 2, "the answer is recorded either way");
}
