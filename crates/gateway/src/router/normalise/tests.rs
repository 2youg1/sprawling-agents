// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What normalisation promises: one row per spelling a provider's
//! documentation prints, one property per claim the form relies on.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use proptest::prelude::*;

use super::{DialectHint, HostDefaults, Normalised, normalise};

/// The rows this module's tests need from the preset table, stated
/// here so a case reads without opening another file. The real table
/// is `provider::preset`, and nothing in the algorithm depends on
/// which rows it holds.
struct Presets;

impl HostDefaults for Presets {
    fn default_path(&self, host: &str) -> Option<&str> {
        match host {
            "openrouter.ai" => Some("/api/v1"),
            "generativelanguage.googleapis.com" => Some("/v1beta"),
            "api.anthropic.com" | "api.openai.com" => Some("/v1"),
            _ => None,
        }
    }

    fn default_dialect(&self, host: &str) -> Option<DialectHint> {
        match host {
            "api.anthropic.com" => Some(DialectHint::Messages),
            _ => None,
        }
    }
}

fn resolved(entered: &str, hint: DialectHint) -> Normalised {
    normalise(entered, hint, &Presets).unwrap()
}

fn assert_resolves(entered: &str, hint: DialectHint, base_url: &str, dialect: DialectHint) {
    let got = resolved(entered, hint);
    assert_eq!(got.base_url, base_url, "base url of {entered}");
    assert_eq!(got.dialect, dialect, "dialect of {entered}");
}

#[test]
fn a_pasted_chat_url_gives_back_the_base_and_the_chat_shape() {
    assert_resolves(
        "https://x.test/v1/chat/completions",
        DialectHint::Unset,
        "https://x.test/v1",
        DialectHint::Chat,
    );
}

#[test]
fn a_pasted_responses_url_gives_back_the_base_and_the_responses_shape() {
    assert_resolves(
        "https://x.test/v1/responses",
        DialectHint::Unset,
        "https://x.test/v1",
        DialectHint::Responses,
    );
}

#[test]
fn a_pasted_messages_url_gives_back_the_base_and_the_messages_shape() {
    assert_resolves(
        "https://x.test/v1/messages",
        DialectHint::Unset,
        "https://x.test/v1",
        DialectHint::Messages,
    );
}

#[test]
fn a_host_alone_takes_the_path_and_keeps_the_shape_the_person_chose() {
    for entered in ["https://x.test", "https://x.test/", "https://x.test/v1/"] {
        assert_resolves(
            entered,
            DialectHint::Responses,
            "https://x.test/v1",
            DialectHint::Responses,
        );
    }
}

#[test]
fn a_documented_anthropic_host_takes_its_path_and_its_shape_from_the_preset_table() {
    assert_resolves(
        "https://api.anthropic.com",
        DialectHint::Unset,
        "https://api.anthropic.com/v1",
        DialectHint::Messages,
    );
}

#[test]
fn a_url_without_a_scheme_is_called_over_https() {
    assert_resolves(
        "x.test/v1",
        DialectHint::Chat,
        "https://x.test/v1",
        DialectHint::Chat,
    );
}

#[test]
fn a_server_on_this_machine_is_called_over_http_in_the_openai_shape() {
    assert_resolves(
        "http://127.0.0.1:11434",
        DialectHint::Unset,
        "http://127.0.0.1:11434/v1",
        DialectHint::Chat,
    );
    assert_resolves(
        "localhost:11434",
        DialectHint::Unset,
        "http://localhost:11434/v1",
        DialectHint::Chat,
    );
}

#[test]
fn a_listed_host_keeps_the_path_it_serves_at_rather_than_v1() {
    assert_resolves(
        "https://openrouter.ai",
        DialectHint::Chat,
        "https://openrouter.ai/api/v1",
        DialectHint::Chat,
    );
    assert_resolves(
        "https://generativelanguage.googleapis.com",
        DialectHint::Chat,
        "https://generativelanguage.googleapis.com/v1beta",
        DialectHint::Chat,
    );
}

#[test]
fn a_path_the_person_entered_is_never_rewritten() {
    assert_resolves(
        "https://x.test/v1beta",
        DialectHint::Chat,
        "https://x.test/v1beta",
        DialectHint::Chat,
    );
    assert_resolves(
        "https://relay.test/openai/v1/chat/completions",
        DialectHint::Unset,
        "https://relay.test/openai/v1",
        DialectHint::Chat,
    );
}

#[test]
fn the_url_outranks_a_toggle_that_disagrees_with_it() {
    assert_resolves(
        "https://x.test/v1/messages",
        DialectHint::Chat,
        "https://x.test/v1",
        DialectHint::Messages,
    );
}

#[test]
fn a_scheme_the_city_cannot_call_is_refused_with_a_way_out() {
    let refused = normalise("ftp://x.test/v1", DialectHint::Unset, &Presets).unwrap_err();
    assert_eq!(*refused.code(), kernel::AxCode::ConfigInvalid);
    assert!(
        !refused.recovery().is_empty(),
        "a refusal a person reads on a form carries the next step"
    );
}

#[test]
fn a_url_with_no_host_is_refused() {
    assert!(normalise("   ", DialectHint::Unset, &Presets).is_err());
    assert!(normalise("https://", DialectHint::Unset, &Presets).is_err());
}

fn hint() -> impl Strategy<Value = DialectHint> {
    prop_oneof![
        Just(DialectHint::Unset),
        Just(DialectHint::Chat),
        Just(DialectHint::Responses),
        Just(DialectHint::Messages),
    ]
}

fn host() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("x.test".to_owned()),
        Just("api.anthropic.com".to_owned()),
        Just("openrouter.ai".to_owned()),
        Just("generativelanguage.googleapis.com".to_owned()),
        Just("127.0.0.1:11434".to_owned()),
        Just("localhost".to_owned()),
    ]
}

fn segment() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("v1".to_owned()),
        Just("api".to_owned()),
        Just("v1beta".to_owned()),
        Just("openai".to_owned()),
        Just("chat".to_owned()),
        Just("completions".to_owned()),
        Just("messages".to_owned()),
        Just("responses".to_owned()),
    ]
}

/// A URL assembled the way people actually paste them: with or without
/// a scheme, with or without a trailing slash, and with a path that may
/// end in a face or in nonsense.
fn entered_url() -> impl Strategy<Value = String> {
    (
        prop_oneof![Just(""), Just("http://"), Just("https://")],
        host(),
        proptest::collection::vec(segment(), 0..4),
        prop_oneof![Just(""), Just("/")],
    )
        .prop_map(|(scheme, host, segments, slash)| {
            let path = segments.join("/");
            if path.is_empty() {
                format!("{scheme}{host}{slash}")
            } else {
                format!("{scheme}{host}/{path}{slash}")
            }
        })
}

proptest! {
    /// Normalising the answer again leaves it alone. The form shows the
    /// result in the same box the person typed in, so a second edit
    /// normalises what the first one produced.
    #[test]
    fn normalising_the_answer_again_changes_nothing(entered in entered_url(), hint in hint()) {
        let once = normalise(&entered, hint, &Presets).unwrap();
        let twice = normalise(&once.base_url, once.dialect, &Presets).unwrap();
        prop_assert_eq!(&twice.base_url, &once.base_url);
        prop_assert_eq!(twice.dialect, once.dialect);
    }

    /// Every spelling of one endpoint lands on one registration: that
    /// is the whole reason this function exists.
    #[test]
    fn every_spelling_of_one_endpoint_lands_on_one_answer(
        host in prop_oneof![Just("x.test"), Just("relay.test")],
        prefix in prop_oneof![Just(""), Just("/openai")],
        slash in any::<bool>(),
    ) {
        let tail = if slash { "/" } else { "" };
        let upper = host.to_ascii_uppercase();
        let spellings = [
            format!("https://{host}{prefix}/v1{tail}"),
            format!("https://{host}{prefix}/v1/chat/completions{tail}"),
            format!("{host}{prefix}/v1{tail}"),
            format!("  https://{host}{prefix}/v1{tail}  "),
            format!("HTTPS://{upper}{prefix}/v1{tail}"),
        ];
        for spelling in &spellings {
            let got = normalise(spelling, DialectHint::Chat, &Presets).unwrap();
            prop_assert_eq!(got.base_url, format!("https://{host}{prefix}/v1"));
            prop_assert_eq!(got.dialect, DialectHint::Chat);
        }
    }
}
