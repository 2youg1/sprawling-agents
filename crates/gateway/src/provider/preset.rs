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
//! statement by one vendor.

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
/// The path and shape columns are read by the `HostDefaults` answer
/// that `router::normalise` takes, which lands with the attach caller
/// in `bin::assembly`; until then only the ceiling ladder reads this
/// table, and the expectation below fails the build on the day that
/// changes — which is when the attribute goes.
pub struct HostPreset {
    pub host: &'static str,
    /// The path the API hangs under, leading slash included. Filled in
    /// only when the person entered a host with no path at all.
    pub base_path: &'static str,
    /// The shape this host answers in, when neither the pasted URL nor
    /// the person said. `None` where a host serves several shapes and
    /// choosing one for the person would be a guess with a 404 in it.
    ///
    /// Spelled in the enum a registration is stored under, so that the
    /// table states what it would register rather than a second
    /// vocabulary for the same two shapes.
    pub dialect: Option<DialectKind>,
    /// The models this vendor documents, longest matching id prefix
    /// first at lookup time. Empty where the endpoint states its own
    /// facts in its model list, which outranks this table anyway.
    pub models: &'static [ModelPreset],
    /// Where `base_path` and `dialect` were read.
    pub source: &'static str,
}

/// What one vendor documents about a family of models.
///
/// Matched by id prefix rather than by exact id: a vendor prints
/// `claude-sonnet-4-5-20250929` and serves it as `claude-sonnet-4-5`
/// and as `claude-sonnet-4-5-latest`, and three rows for one
/// documented figure is three chances to update two of them.
///
/// The window and modality columns reach a registration through the
/// same caller as the host columns above.
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

/// The row for one model at one base URL.
///
/// The host decides which vendor's rows are consulted, so a proxy that
/// serves `claude-sonnet-4-5` under its own name is not handed
/// Anthropic's figures: what that proxy truncates at is that proxy's
/// fact, and its model list is where it states it.
///
/// The longest matching prefix wins, so a row for a family and a row
/// for one member of it can both stand and the more specific one
/// answers.
#[must_use]
pub fn model_for(base_url: &str, id: &str) -> Option<&'static ModelPreset> {
    let (host, _, _) = crate::reach::split(base_url)?;
    let rows = for_host(&host)?;
    rows.models
        .iter()
        .filter(|row| id.starts_with(row.id_prefix))
        .max_by_key(|row| row.id_prefix.len())
}

/// The ceiling this table states for one model, as a ceiling rather
/// than as a count.
#[must_use]
pub fn ceiling_for(base_url: &str, id: &str) -> Option<Ceiling> {
    Ceiling::new(model_for(base_url, id)?.max_output_tokens)
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
    fn default_path(&self, host: &str) -> Option<&str> {
        for_host(host).map(|row| row.base_path)
    }

    fn default_dialect(&self, host: &str) -> Option<crate::router::DialectHint> {
        match for_host(host)?.dialect? {
            kernel::DialectKind::Anthropic => Some(crate::router::DialectHint::Messages),
            kernel::DialectKind::OpenAi => Some(crate::router::DialectHint::Chat),
            kernel::DialectKind::OpenAiResponses => Some(crate::router::DialectHint::Responses),
        }
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
        Some(for_host(host)?.base_path)
    }

    #[test]
    fn a_listed_host_states_the_path_it_serves_under() {
        assert_eq!(path_of("openrouter.ai"), Some("/api/v1"));
        assert_eq!(path_of("api.kimi.com"), Some("/coding/v1"));
        assert_eq!(path_of("api.moonshot.cn"), Some("/v1"));
        assert_eq!(
            path_of("generativelanguage.googleapis.com"),
            Some("/v1beta")
        );
        assert_eq!(path_of("api.anthropic.com"), Some("/v1"));
        assert_eq!(path_of("llm.example.test"), None);
    }

    #[test]
    fn a_host_that_serves_one_shape_says_so_and_one_that_serves_two_does_not() {
        assert_eq!(
            for_host("api.anthropic.com").and_then(|row| row.dialect),
            Some(DialectKind::Anthropic)
        );
        assert_eq!(for_host("api.openai.com").and_then(|row| row.dialect), None);
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

    /// A vendor's figures belong to that vendor's host. A relay that
    /// serves the same id truncates where the relay says it does, and
    /// its model list is the place that says so.
    #[test]
    fn a_relay_serving_the_same_id_is_not_handed_the_vendors_figures() {
        assert!(model_for("https://relay.example.test/v1", "claude-sonnet-4-5").is_none());
        assert!(ceiling_for("https://relay.example.test/v1", "claude-sonnet-4-5").is_none());
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
                let base_url = format!("https://{}{}", row.host, row.base_path);
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
            assert!(row.base_path.starts_with('/'), "{}", row.host);
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
}
