// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod rows;

pub use rows::PRESETS;

use kernel::{Ceiling, DialectKind};

use crate::market::InputKinds;

pub struct HostPreset {
    pub host: &'static str,

    pub faces: &'static [Face],

    pub models: &'static [ModelPreset],

    pub chat: ChatSpelling,

    pub session_header: Option<&'static str>,

    pub cache_key_field: Option<&'static str>,

    pub generation: GenerationPreset,

    pub source: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct GenerationPreset {
    pub anthropic: kernel::GenerationSupport,
    pub open_ai: kernel::GenerationSupport,
    pub responses: kernel::GenerationSupport,
}

impl GenerationPreset {
    pub const UNKNOWN: GenerationPreset = GenerationPreset {
        anthropic: kernel::GenerationSupport {
            temperature: None,
            top_p: None,
            top_k: None,
        },
        open_ai: kernel::GenerationSupport {
            temperature: None,
            top_p: None,
            top_k: None,
        },
        responses: kernel::GenerationSupport {
            temperature: None,
            top_p: None,
            top_k: None,
        },
    };
}

pub(crate) const ANTHROPIC_GENERATION: kernel::GenerationSupport = kernel::GenerationSupport {
    temperature: Some(kernel::ParameterSupport::Supported),
    top_p: Some(kernel::ParameterSupport::Supported),
    top_k: Some(kernel::ParameterSupport::Supported),
};

pub(crate) const OPENAI_GENERATION: kernel::GenerationSupport = kernel::GenerationSupport {
    temperature: None,
    top_p: None,
    top_k: Some(kernel::ParameterSupport::Unsupported),
};

pub struct Face {
    pub dialect: DialectKind,
    pub path: &'static str,
}

impl HostPreset {
    #[must_use]
    pub fn default_dialect(&self) -> Option<DialectKind> {
        match self.faces {
            [only] => Some(only.dialect),
            _ => None,
        }
    }

    #[must_use]
    pub fn path_for(&self, dialect: Option<DialectKind>) -> Option<&'static str> {
        dialect
            .and_then(|chosen| self.faces.iter().find(|face| face.dialect == chosen))
            .or_else(|| self.faces.first())
            .map(|face| face.path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownHost {
    pub host: &'static str,
    pub faces: Vec<(DialectKind, String)>,
}

pub fn known_hosts() -> Result<Vec<KnownHost>, kernel::AxError> {
    PRESETS
        .iter()
        .map(|row| {
            let faces = row
                .faces
                .iter()
                .map(|face| {
                    let stored = crate::router::normalise_entered(
                        row.host,
                        crate::router::DialectHint::of(face.dialect),
                    )?;
                    Ok((face.dialect, stored.base_url))
                })
                .collect::<Result<Vec<_>, kernel::AxError>>()?;
            Ok(KnownHost {
                host: row.host,
                faces,
            })
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChatSpelling {
    pub ceiling: CeilingField,
    pub effort: EffortField,
    pub reasoning: ReasoningReturn,
    pub chat_tool_pictures: ToolPictureCarry,
    pub responses_tool_pictures: ToolPictureCarry,
}

impl ChatSpelling {
    pub const DOCUMENTED: ChatSpelling = ChatSpelling {
        ceiling: CeilingField::MaxTokens,
        effort: EffortField::ReasoningEffort,
        reasoning: ReasoningReturn::Dropped,
        chat_tool_pictures: ToolPictureCarry::FollowingUser,
        responses_tool_pictures: ToolPictureCarry::FollowingUser,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolPictureCarry {
    FollowingUser,

    InToolContent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CeilingField {
    MaxTokens,

    MaxCompletionTokens,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffortField {
    ReasoningEffort,

    ReasoningObject,

    ThinkingToggle,

    EnableThinking,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningReturn {
    Dropped,

    AsReasoningContent,
}

pub struct ModelPreset {
    pub id_prefix: &'static str,
    pub context_tokens: u64,

    pub max_output_tokens: u64,
    pub input: InputKinds,

    pub image_max_side: Option<std::num::NonZeroU32>,

    pub thinking: Option<PresetThinking>,

    pub cache_min_tokens: Option<u64>,

    pub source: &'static str,
}

pub struct PresetThinking {
    pub levels: &'static [kernel::Effort],

    pub default: Option<kernel::Effort>,

    pub source: &'static str,
}

#[must_use]
pub fn for_host(host: &str) -> Option<&'static HostPreset> {
    PRESETS.iter().find(|row| row.host == host)
}

#[must_use]
pub fn chat_spelling(base_url: &str) -> ChatSpelling {
    host_row(base_url).map_or(ChatSpelling::DOCUMENTED, |row| row.chat)
}

#[must_use]
pub fn session_header(base_url: &str) -> Option<&'static str> {
    host_row(base_url)?.session_header
}

#[must_use]
pub(crate) fn cache_key_field(base_url: &str) -> Option<&'static str> {
    host_row(base_url)?.cache_key_field
}

fn host_row(base_url: &str) -> Option<&'static HostPreset> {
    let (host, _, _) = crate::reach::split(base_url)?;
    for_host(&host)
}

#[must_use]
pub fn model_for(base_url: &str, id: &str) -> Option<&'static ModelPreset> {
    row_for(
        host_row(base_url).map_or(&[][..], |row| row.models),
        PRESETS.iter().flat_map(|row| row.models.iter()),
        crate::reach::is_local(base_url),
        id,
    )
}

fn row_for<'rows>(
    own: &'rows [ModelPreset],
    vendors: impl Iterator<Item = &'rows ModelPreset>,
    local: bool,
    id: &str,
) -> Option<&'rows ModelPreset> {
    let relayed = own.is_empty() && !local;
    own.iter()
        .chain(vendors.filter(|_| relayed))
        .filter(|row| id.starts_with(row.id_prefix))
        .max_by_key(|row| row.id_prefix.len())
}

pub(crate) const CACHE_MIN_TOKENS_DEFAULT: u64 = 4_096;

#[must_use]
pub(crate) fn cache_min_tokens_for(base_url: &str, id: &str) -> u64 {
    model_for(base_url, id)
        .and_then(|row| row.cache_min_tokens)
        .unwrap_or(CACHE_MIN_TOKENS_DEFAULT)
}

#[must_use]
pub fn ceiling_for(base_url: &str, id: &str) -> Option<Ceiling> {
    Ceiling::new(model_for(base_url, id)?.max_output_tokens)
}

#[must_use]
pub fn window_for(base_url: &str, id: &str) -> Option<kernel::Window> {
    kernel::Window::new(model_for(base_url, id)?.context_tokens)
}

#[must_use]
pub(crate) fn input_for(base_url: &str, id: &str) -> Option<InputKinds> {
    model_for(base_url, id).map(|row| row.input)
}

#[must_use]
pub(crate) fn image_max_side(
    base_url: &str,
    id: &str,
    picture_parts: usize,
) -> Option<std::num::NonZeroU32> {
    let row = model_for(base_url, id)?;
    let ceiling = row.image_max_side?;
    if row.id_prefix.starts_with("deepseek-") && picture_parts >= 15 {
        std::num::NonZeroU32::new(4096).map(|limit| ceiling.min(limit))
    } else {
        Some(ceiling)
    }
}

#[must_use]
pub(crate) fn thinking_for(base_url: &str, id: &str) -> Option<&'static PresetThinking> {
    model_for(base_url, id)?.thinking.as_ref()
}

pub(crate) fn generation_for(base_url: &str, dialect: DialectKind) -> kernel::GenerationSupport {
    let Some(row) = host_row(base_url) else {
        return kernel::GenerationSupport::default();
    };
    match dialect {
        DialectKind::Anthropic => row.generation.anthropic,
        DialectKind::OpenAi => row.generation.open_ai,
        DialectKind::OpenAiResponses => row.generation.responses,
    }
}

pub struct Presets;

impl crate::router::HostDefaults for Presets {
    fn default_path(&self, host: &str, face: crate::router::DialectHint) -> Option<&str> {
        for_host(host)?.path_for(face.dialect())
    }

    fn default_dialect(&self, host: &str) -> Option<crate::router::DialectHint> {
        Some(crate::router::DialectHint::of(
            for_host(host)?.default_dialect()?,
        ))
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
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
            model_for("https://relay.example.test/v1", "claude-sonnet-4-5")
                .map(|row| row.id_prefix),
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
            model_for("https://api.openai.com/v1", "some-unreleased-model")
                .map(|row| row.id_prefix),
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
}
