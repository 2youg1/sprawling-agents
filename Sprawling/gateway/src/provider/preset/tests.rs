// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn path_of(host: &str) -> Option<&'static str> {
    for_host(host)?.path_for(None)
}

#[test]
fn a_listed_host_states_the_path_it_serves_under() {
    assert_eq!(path_of("openrouter.ai"), Some("/api/v1"));
    assert_eq!(path_of("api.kimi.com"), Some("/coding/v1"));
    assert_eq!(path_of("api.moonshot.cn"), Some("/v1"));
    assert_eq!(
        path_of("generativelanguage.googleapis.com"),
        Some("/v1beta/openai")
    );
    assert_eq!(path_of("api.deepseek.com"), Some("/"));
    assert_eq!(path_of("open.bigmodel.cn"), Some("/api/paas/v4"));
    assert_eq!(path_of("api.anthropic.com"), Some("/v1"));
    assert_eq!(path_of("llm.example.test"), None);
}

#[test]
fn a_bare_host_reaches_the_chat_face_its_vendor_documents() {
    use crate::router::{DialectHint, normalise_entered};
    let chat_url = |entered: &str| {
        let stored = normalise_entered(entered, DialectHint::Chat).unwrap();
        crate::router::join(&stored.base_url, "chat/completions")
    };
    assert_eq!(
        chat_url("api.deepseek.com"),
        "https://api.deepseek.com/chat/completions"
    );
    assert_eq!(
        normalise_entered("api.deepseek.com", DialectHint::Chat)
            .unwrap()
            .base_url,
        "https://api.deepseek.com",
        "a stored base URL carries no trailing slash"
    );
    assert_eq!(
        chat_url("https://generativelanguage.googleapis.com"),
        "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
    );
}

#[test]
fn a_bare_host_reaches_the_path_of_the_face_the_person_chose() {
    use crate::router::{DialectHint, normalise_entered};
    let base =
        |entered: &str, face: DialectHint| normalise_entered(entered, face).unwrap().base_url;
    assert_eq!(
        base("api.deepseek.com", DialectHint::Messages),
        "https://api.deepseek.com/anthropic/v1"
    );
    assert_eq!(
        base("api.deepseek.com", DialectHint::Chat),
        "https://api.deepseek.com"
    );
    assert_eq!(
        base("api.kimi.com", DialectHint::Messages),
        "https://api.kimi.com/coding/v1"
    );
}

#[test]
fn the_chat_spelling_and_the_session_header_are_the_hosts() {
    assert_eq!(
        chat_spelling("https://api.openai.com/v1").ceiling,
        CeilingField::MaxCompletionTokens
    );
    assert_eq!(
        chat_spelling("https://api.deepseek.com").reasoning,
        ReasoningReturn::AsReasoningContent
    );
    assert_eq!(
        chat_spelling("https://api.deepseek.com").chat_tool_pictures,
        ToolPictureCarry::FollowingUser
    );
    assert_eq!(
        chat_spelling("https://api.deepseek.com").responses_tool_pictures,
        ToolPictureCarry::InToolContent
    );

    for base in [
        "https://api.moonshot.ai/v1",
        "https://api.moonshot.cn/v1",
        "https://api.kimi.com/coding/v1",
    ] {
        assert_eq!(
            chat_spelling(base).chat_tool_pictures,
            ToolPictureCarry::InToolContent,
            "{base}"
        );
    }

    assert_eq!(
        chat_spelling("https://api.openai.com/v1").chat_tool_pictures,
        ToolPictureCarry::FollowingUser
    );
    assert_eq!(
        chat_spelling("https://api.x.ai/v1").chat_tool_pictures,
        ToolPictureCarry::FollowingUser
    );
    assert_eq!(
        chat_spelling("http://127.0.0.1:11434/v1"),
        ChatSpelling::DOCUMENTED
    );
    assert_eq!(
        session_header("https://opencode.ai/zen/go/v1"),
        Some("x-opencode-session")
    );
    assert_eq!(session_header("https://api.deepseek.com"), None);
}

#[test]
fn a_host_that_serves_one_shape_says_so_and_one_that_serves_two_does_not() {
    assert_eq!(
        for_host("api.anthropic.com").and_then(HostPreset::default_dialect),
        Some(DialectKind::Anthropic)
    );
    assert_eq!(
        for_host("api.openai.com").and_then(HostPreset::default_dialect),
        None
    );
    assert!(for_host("llm.example.test").is_none());
}

#[test]
fn a_documented_model_is_matched_by_the_longest_prefix_that_fits() {
    let row = model_for("https://api.anthropic.com/v1", "claude-sonnet-4-5-20250929").unwrap();
    assert_eq!(row.id_prefix, "claude-sonnet-4");
    assert_eq!(row.max_output_tokens, 64_000);
    assert_eq!(row.context_tokens, 200_000);
    assert_eq!(row.input, InputKinds::TextImage);
    assert!(row.source.starts_with("https://"), "every row cites a page");
}

#[test]
fn a_relay_forwarding_a_vendors_id_reads_the_vendors_row_and_a_local_server_does_not() {
    assert_eq!(
        model_for("https://relay.example.test/v1", "claude-sonnet-4-5").map(|row| row.id_prefix),
        Some("claude-sonnet-4")
    );
    assert_eq!(
        window_for("https://relay.example.test/v1", "claude-sonnet-4-5"),
        kernel::Window::new(200_000)
    );
    assert!(model_for("http://127.0.0.1:8000/v1", "claude-sonnet-4-5").is_none());
    assert!(window_for("http://127.0.0.1:8000/v1", "claude-sonnet-4-5").is_none());
}

#[test]
fn the_documented_lineup_is_read_as_the_vendors_page_states_it() {
    let at = |base: &str, id: &str| {
        model_for(base, id).map(|row| (row.context_tokens, row.max_output_tokens))
    };
    let anthropic = "https://api.anthropic.com/v1";
    assert_eq!(at(anthropic, "claude-opus-5-5"), Some((1_000_000, 128_000)));
    assert_eq!(
        at(anthropic, "claude-sonnet-5-5"),
        Some((1_000_000, 128_000))
    );
    assert_eq!(
        at(anthropic, "claude-fable-5-1"),
        Some((1_000_000, 128_000))
    );
    assert_eq!(
        at(anthropic, "claude-haiku-4-5-20251001"),
        Some((200_000, 64_000))
    );
    let deepseek = "https://api.deepseek.com/anthropic";
    assert_eq!(at(deepseek, "deepseek-flash"), Some((1_000_000, 393_216)));
    assert_eq!(at(deepseek, "deepseek-v4-pro"), Some((1_000_000, 393_216)));
}

#[test]
fn deepseek_model_names_keep_their_documented_image_capabilities() {
    let answered = [
        "deepseek-flash",
        "deepseek-v4-flash",
        "deepseek-v4-flash-vision-exp",
        "deepseek-v4-pro",
    ]
    .map(|id| input_for("https://api.deepseek.com", id).unwrap());
    assert_eq!(
        answered,
        [
            InputKinds::TextImage,
            InputKinds::TextImage,
            InputKinds::TextImage,
            InputKinds::Text,
        ]
    );
}

#[test]
fn an_undocumented_model_at_a_documented_host_states_nothing() {
    assert_eq!(
        model_for("https://api.openai.com/v1", "some-unreleased-model").map(|row| row.id_prefix),
        None
    );
}

#[test]
fn no_pinned_catalogue_row_is_also_matched_by_this_table() {
    let catalogue = crate::market::MarketSnapshot::builtin().unwrap();
    for row in PRESETS {
        for model in row.models {
            let base_url = format!("https://{}{}", row.host, row.path_for(None).unwrap());
            assert!(
                catalogue.lookup(model.id_prefix).is_none(),
                "{} is pinned in the catalogue and matched here",
                model.id_prefix
            );
            assert!(
                ceiling_for(&base_url, model.id_prefix).is_some(),
                "{} states a ceiling this table can hand out",
                model.id_prefix
            );
        }
    }
}

#[test]
fn every_row_states_a_ceiling_that_can_be_called_with() {
    for row in PRESETS {
        for model in row.models {
            assert!(model.max_output_tokens > 0, "{}", model.id_prefix);
            assert!(
                model.context_tokens > model.max_output_tokens,
                "{} writes more than it can read",
                model.id_prefix
            );
            assert!(model.source.starts_with("https://"), "{}", model.id_prefix);
        }
    }
}

#[test]
fn every_host_row_cites_the_page_it_was_read_from() {
    for row in PRESETS {
        assert!(row.source.starts_with("https://"), "{}", row.host);
        assert!(!row.faces.is_empty(), "{} names no face", row.host);
        for face in row.faces {
            assert!(face.path.starts_with('/'), "{}", row.host);
        }
    }
}

#[test]
fn every_known_host_offers_each_face_at_the_address_attaching_stores() {
    let hosts = known_hosts().unwrap();
    assert_eq!(hosts.len(), PRESETS.len());
    let deepseek = hosts
        .iter()
        .find(|row| row.host == "api.deepseek.com")
        .unwrap();
    assert_eq!(
        deepseek.faces,
        vec![
            (DialectKind::OpenAi, "https://api.deepseek.com".to_owned()),
            (
                DialectKind::OpenAiResponses,
                "https://api.deepseek.com".to_owned()
            ),
            (
                DialectKind::Anthropic,
                "https://api.deepseek.com/anthropic/v1".to_owned()
            ),
        ]
    );
    for row in &hosts {
        for (dialect, base_url) in &row.faces {
            let again = crate::router::normalise_entered(
                base_url,
                crate::router::DialectHint::of(*dialect),
            )
            .unwrap();
            assert_eq!(&again.base_url, base_url, "{}", row.host);
        }
    }
}

#[test]
fn one_host_has_one_row() {
    let mut hosts: Vec<&str> = PRESETS.iter().map(|row| row.host).collect();
    hosts.sort_unstable();
    let listed = hosts.len();
    hosts.dedup();
    assert_eq!(hosts.len(), listed, "a host with two rows is two answers");
}

fn row(id_prefix: &'static str) -> ModelPreset {
    ModelPreset {
        id_prefix,
        context_tokens: 1,
        max_output_tokens: 1,
        input: InputKinds::Text,
        image_max_side: None,
        thinking: None,
        cache_min_tokens: None,
        source: "test",
    }
}

fn prefixes() -> impl proptest::strategy::Strategy<Value = Vec<&'static str>> {
    proptest::collection::vec(
        proptest::sample::select(vec!["", "a", "ab", "abc", "b", "ba", "c"]),
        0..6,
    )
}

proptest::proptest! {

    #[test]
    fn the_row_rule_keeps_the_lean_properties(
        own in prefixes(),
        vendors in prefixes(),
        local in proptest::bool::ANY,
        id in "[abc]{0,4}",
    ) {
        let own: Vec<ModelPreset> = own.into_iter().map(row).collect();
        let vendors: Vec<ModelPreset> = vendors.into_iter().map(row).collect();
        let found = row_for(&own, vendors.iter(), local, &id);

        if own.is_empty() && local {
            proptest::prop_assert!(found.is_none());
        }
        if let Some(answer) = found {

            if !own.is_empty() {
                proptest::prop_assert!(own.iter().any(|mine| std::ptr::eq(mine, answer)));
            }

            proptest::prop_assert!(id.starts_with(answer.id_prefix));
        }
    }
}
