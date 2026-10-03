// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a provider's own documentation says, written down once.
//!
//! **Most endpoints answer a model list that states nothing but an id.**
//! The provider's documentation states the rest — the path the API hangs
//! under, the request shape, the window and the output ceiling of each
//! model it names — and without this table those facts reach the city
//! only by a person typing them into a form.
//!
//! Every row cites the page it was read from, because a pinned figure
//! whose source nobody can open is a figure nobody can recheck. A fact
//! no page states is absent from the row: the ladder in
//! [`super::ceiling`] then falls to its next rung, which is what "do not
//! invent a number" means in code.
//!
//! Two readers share this one table. [`for_host`] answers the
//! normalisation algorithm's two questions about a host, so that
//! `router::normalise` carries no host names of its own; [`model_for`]
//! answers the ceiling ladder's question about one model id. Host
//! defaults and model facts are one table because they are one
//! statement by one vendor. Which row answers for an id is modelled in
//! `crates/gateway/spec/Provider/Preset.lean`; the table's interface is
//! `crates/gateway/spec/Provider.lean` §8-17.

mod rows;

pub use rows::PRESETS;

use kernel::{Ceiling, DialectKind};

use crate::market::InputKinds;

/// What one vendor serves, and where.
///
/// `host` is the authority a base URL carries, lower-cased and without
/// a port — the spelling `reach::split` produces and `normalise` asks
/// with.
///
/// The faces column is read by the `HostDefaults` answer that
/// `router::normalise` takes, and by [`known_hosts`], which is what the
/// settings page offers a person to pick from.
pub struct HostPreset {
    pub host: &'static str,
    /// The faces this host's documentation says it answers on, and the
    /// path each hangs under, the vendor's main face first. Spelled in
    /// the enum a registration is stored under, so that the table
    /// states what it would register rather than a second vocabulary
    /// for the same shapes.
    pub faces: &'static [Face],
    /// The models this vendor documents, longest matching id prefix
    /// first at lookup time. Empty where the endpoint states its own
    /// facts in its model list, which outranks this table anyway.
    pub models: &'static [ModelPreset],
    /// How this vendor spells what its chat face leaves to the vendor.
    pub chat: ChatSpelling,
    /// The header this vendor asks to carry one conversation's id, so
    /// that its routing and prompt cache see a conversation as one.
    pub session_header: Option<&'static str>,
    /// Where `faces`, `chat` and `session_header` were read.
    pub source: &'static str,
}

/// One face a host answers on, and the path it hangs under, leading
/// slash included. The path is filled in only when the person entered
/// a host with no path at all.
pub struct Face {
    pub dialect: DialectKind,
    pub path: &'static str,
}

impl HostPreset {
    /// The shape this host answers in when neither the pasted URL nor
    /// the person said: its one face, or `None` where it serves several
    /// and choosing one for the person would be a guess with a 404 in
    /// it.
    #[must_use]
    pub fn default_dialect(&self) -> Option<DialectKind> {
        match self.faces {
            [only] => Some(only.dialect),
            _ => None,
        }
    }

    /// The path the face the person chose hangs under. A face this host
    /// does not list, or no face chosen yet, takes the main face's
    /// path: the vendor put that face first.
    #[must_use]
    pub fn path_for(&self, dialect: Option<DialectKind>) -> Option<&'static str> {
        dialect
            .and_then(|chosen| self.faces.iter().find(|face| face.dialect == chosen))
            .or_else(|| self.faces.first())
            .map(|face| face.path)
    }
}

/// One host the settings page offers, with the base URL each of its
/// faces is called at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownHost {
    pub host: &'static str,
    pub faces: Vec<(DialectKind, String)>,
}

/// Every host this table lists, each face's base URL computed by the
/// same normalisation an attach runs, so the address a form fills is
/// the address the city would store.
///
/// # Errors
/// A row whose host the normaliser refuses, which is a defect of this
/// table that a test holds shut.
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

/// The three fields of the OpenAI-compatible chat face whose spelling
/// each vendor's documentation decides for itself.
///
/// One value because the three are read together, at the one place a
/// chat request is written, and a host that states one states all
/// three.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChatSpelling {
    pub ceiling: CeilingField,
    pub effort: EffortField,
    pub reasoning: ReasoningReturn,
}

impl ChatSpelling {
    /// What a host this table does not list is sent.
    ///
    /// `max_tokens` although the OpenAI specification deprecates it:
    /// local inference servers and most compatible servers read
    /// only that name, and a ceiling under a name a server ignores is a
    /// ceiling silently dropped. `reasoning_effort` because it is the
    /// specification's own field. An earlier turn's reasoning is not
    /// sent back, because the specification's assistant message has no
    /// field that carries it.
    pub const DOCUMENTED: ChatSpelling = ChatSpelling {
        ceiling: CeilingField::MaxTokens,
        effort: EffortField::ReasoningEffort,
        reasoning: ReasoningReturn::Dropped,
    };
}

/// Which field carries the output ceiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CeilingField {
    MaxTokens,
    /// The specification's replacement; OpenAI's reasoning models
    /// answer 400 to `max_tokens`.
    MaxCompletionTokens,
}

/// Which field carries how hard to think.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffortField {
    /// A string at the top level, as the specification spells it.
    ReasoningEffort,
    /// `reasoning: { effort }`, OpenRouter's own parameter and the one
    /// its documentation lets carry `max`.
    ReasoningObject,
}

/// Whether an earlier turn's reasoning goes back to the provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningReturn {
    Dropped,
    /// As `reasoning_content` on the assistant message it came with.
    /// DeepSeek answers 400 to a request with tools whose history
    /// lacks it.
    AsReasoningContent,
}

/// What one vendor documents about a family of models.
///
/// Matched by id prefix rather than by exact id: a vendor prints
/// `claude-sonnet-4-5-20250929` and serves it as `claude-sonnet-4-5`
/// and as `claude-sonnet-4-5-latest`, and three rows for one
/// documented figure is three chances to update two of them.
///
/// The window and modality columns reach a registration through the
/// same caller as the host columns above: the window through
/// [`window_for`], the modality through the ladder in [`super::input`].
pub struct ModelPreset {
    pub id_prefix: &'static str,
    pub context_tokens: u64,
    /// Non-zero by the table's own rule: zero is not a ceiling, and a
    /// row that cannot state one states nothing instead.
    pub max_output_tokens: u64,
    pub input: InputKinds,
    /// Where the two figures were read.
    pub source: &'static str,
}

/// The row for one host, or `None` when this city has never been told
/// what that host serves.
#[must_use]
pub fn for_host(host: &str) -> Option<&'static HostPreset> {
    PRESETS.iter().find(|row| row.host == host)
}

/// How the chat face at one base URL is spelled.
#[must_use]
pub fn chat_spelling(base_url: &str) -> ChatSpelling {
    host_row(base_url).map_or(ChatSpelling::DOCUMENTED, |row| row.chat)
}

/// The header the vendor at one base URL asks a conversation's id in.
#[must_use]
pub fn session_header(base_url: &str) -> Option<&'static str> {
    host_row(base_url)?.session_header
}

fn host_row(base_url: &str) -> Option<&'static HostPreset> {
    let (host, _, _) = crate::reach::split(base_url)?;
    for_host(&host)
}

/// The row for one model at one base URL.
///
/// The host's own rows answer first. A host with no model rows of its
/// own that is not on this machine - a relay forwarding a vendor's id -
/// is answered from the rows of the vendor that published the id:
/// relays rarely state a ceiling in their model list, and the vendor's
/// documented figure is closer to the relay's fact than a policy figure
/// that belongs to neither (`crates/gateway/spec/Provider.lean` §8-17). A server on this
/// machine borrows nothing, because the window of a model it serves is
/// its own configuration.
///
/// The longest matching prefix wins, so a row for a family and a row
/// for one member of it can both stand and the more specific one
/// answers.
#[must_use]
pub fn model_for(base_url: &str, id: &str) -> Option<&'static ModelPreset> {
    row_for(
        host_row(base_url).map_or(&[][..], |row| row.models),
        PRESETS.iter().flat_map(|row| row.models.iter()),
        crate::reach::is_local(base_url),
        id,
    )
}

/// Which row answers for `id`, over any row table:
/// `Gateway.Provider.Preset.model_for` with `own` the host's own model
/// rows, `vendors` every model row in table order, and `local` whether
/// the base URL is on this machine. The table is a parameter so a check
/// can drive the rule over tables [`PRESETS`] does not hold.
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

/// The ceiling this table states for one model, as a ceiling rather
/// than as a count.
#[must_use]
pub fn ceiling_for(base_url: &str, id: &str) -> Option<Ceiling> {
    Ceiling::new(model_for(base_url, id)?.max_output_tokens)
}

/// The context window this table states for one model, as a window
/// rather than as a count.
#[must_use]
pub fn window_for(base_url: &str, id: &str) -> Option<kernel::Window> {
    kernel::Window::new(model_for(base_url, id)?.context_tokens)
}

/// What this table states one model accepts. Read only by the ladder
/// in [`super::input`], which decides whether this rung answers.
#[must_use]
pub(crate) fn input_for(base_url: &str, id: &str) -> Option<InputKinds> {
    model_for(base_url, id).map(|row| row.input)
}

/// The preset table as the host authority the normaliser reads.
///
/// A zero-sized handle rather than a free function, because
/// `router::normalise` must not know that a table exists at all: it
/// asks whoever it was handed what a host serves, and this is the
/// answer the shipped build hands it. The mapping from a stored
/// `DialectKind` to the hint the form speaks is here and nowhere else.
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

    /// A bare host becomes the URL the vendor's documentation prints:
    /// the chat face straight under the host for DeepSeek, and under
    /// `/openai` for Gemini, whose `/v1beta` alone is another shape.
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

    /// A host that hangs its Anthropic-compatible face under another
    /// path than its OpenAI one is reached on the path of the face the
    /// person chose (`crates/gateway/spec/Provider.lean` §8-17).
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

    /// A relay's model list rarely states a ceiling, so the vendor that
    /// published the id it forwards answers for it; a server on this
    /// machine does not borrow the vendor's figures, because its window
    /// is its own configuration.
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

    /// The current lineup on the vendor's own page, and DeepSeek's two
    /// families, whose ceiling the API reference states exactly.
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
    fn an_undocumented_model_at_a_documented_host_states_nothing() {
        assert_eq!(
            model_for("https://api.openai.com/v1", "some-unreleased-model")
                .map(|row| row.id_prefix),
            None
        );
    }

    /// The pinned catalogue and this table are one rung of the ladder
    /// reached through two indexes — an exact id and a host with a
    /// prefix — so no id may reach both. One fact, one home.
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

    /// Zero is not a ceiling: the Anthropic wire refuses it and the
    /// OpenAI wire sends a request that answers with no content.
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

    /// The page is offered one base URL per face, and it is the one
    /// attaching would store, so a vendor picked on the page is the
    /// vendor the city calls.
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

    /// A model row with only the prefix this rule reads.
    fn row(id_prefix: &'static str) -> ModelPreset {
        ModelPreset {
            id_prefix,
            context_tokens: 1,
            max_output_tokens: 1,
            input: InputKinds::Text,
            source: "test",
        }
    }

    /// The prefixes of a random row table; rows are built in the test,
    /// because `ModelPreset` carries no `Debug` for proptest to print.
    fn prefixes() -> impl proptest::strategy::Strategy<Value = Vec<&'static str>> {
        proptest::collection::vec(
            proptest::sample::select(vec!["", "a", "ab", "abc", "b", "ba", "c"]),
            0..6,
        )
    }

    proptest::proptest! {
        /// The three theorems of `crates/gateway/spec/Provider/Preset.lean`
        /// over random row tables and ids, rather than over the shipped
        /// table alone.
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
            // a_server_on_this_machine_borrows_no_vendor_row
            if own.is_empty() && local {
                proptest::prop_assert!(found.is_none());
            }
            if let Some(answer) = found {
                // a_host_with_rows_of_its_own_answers_from_them
                if !own.is_empty() {
                    proptest::prop_assert!(own.iter().any(|mine| std::ptr::eq(mine, answer)));
                }
                // the_answer_is_a_prefix_of_the_id
                proptest::prop_assert!(id.starts_with(answer.id_prefix));
            }
        }
    }
}
