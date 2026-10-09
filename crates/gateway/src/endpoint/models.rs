// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one row of a `/models` answer states, and what it does not.
//!
//! **An OpenAI-shaped model list is an id and whatever else the vendor
//! felt like putting there.** One gateway states `context_length` and
//! `max_completion_tokens`; another nests the same two under
//! `top_provider`; a third states input modalities under
//! `architecture`; most state nothing at all. A reading that insisted
//! on one spelling would report an empty row for every provider but the
//! one it was written against.
//!
//! So this module reads a row as a set of questions, each answered from
//! the first key that carries it, and **absent when no key does**. A
//! figure this city invented would outrank the figure that bills, so
//! nothing here supplies a default, a zero, or a guess.
//!
//! Prices are carried as the provider's own text. Their unit is the
//! provider's too - per token, per million tokens, in dollars or in
//! cents - and a converted number nobody can check against the invoice
//! is worse than the string the provider printed.

use kernel::event::record::{ModelFacts, ThinkingStatement};
use kernel::{Ceiling, Effort};
use serde_json::Value;

/// The keys a context window arrives under, first one wins.
const CONTEXT_KEYS: [&str; 5] = [
    "context_length",
    "context_window",
    "max_context_length",
    "max_context_tokens",
    "max_input_tokens",
];

/// The keys an output ceiling arrives under, first one wins.
///
/// `max_tokens` sits last: a few gateways use it for the window rather
/// than for the ceiling, so it answers only when nothing clearer did.
const OUTPUT_KEYS: [&str; 4] = [
    "max_completion_tokens",
    "max_output_tokens",
    "max_response_tokens",
    "max_tokens",
];

/// The objects a row nests the same facts inside. Read after the row's
/// own keys, so a row that states a figure twice is read at the top.
const NESTS: [&str; 3] = ["top_provider", "limits", "architecture"];

/// A count, from a row's own key or from one of the objects it nests.
///
/// A count spelled as a string (`"128000"`) is read as the count it
/// spells: several gateways quote every number in their catalogue.
fn count(row: &Value, keys: &[&str]) -> Option<u64> {
    for key in keys {
        if let Some(found) = number(row.get(key)) {
            return Some(found);
        }
    }
    for nest in NESTS {
        let inner = row.get(nest)?;
        for key in keys {
            if let Some(found) = number(inner.get(key)) {
                return Some(found);
            }
        }
    }
    None
}

fn number(value: Option<&Value>) -> Option<u64> {
    match value? {
        Value::Number(found) => found.as_u64(),
        Value::String(text) => text.trim().parse::<u64>().ok(),
        Value::Null | Value::Bool(_) | Value::Array(_) | Value::Object(_) => None,
    }
}

/// What the row says the model accepts, sorted and without repeats so
/// two rows stating the same set read the same.
fn modalities(row: &Value) -> Vec<String> {
    let mut found = Vec::new();
    for at in [
        row.get("input_modalities"),
        row.get("modalities").and_then(|held| held.get("input")),
        row.get("architecture")
            .and_then(|held| held.get("input_modalities")),
    ] {
        let Some(Value::Array(list)) = at else {
            continue;
        };
        for entry in list {
            if let Value::String(word) = entry {
                found.push(word.trim().to_ascii_lowercase());
            }
        }
        if !found.is_empty() {
            break;
        }
    }
    found.sort();
    found.dedup();
    found
}

/// A price as the provider printed it. A number is rendered back to its
/// own text rather than through a float, which is how `0.0000004`
/// survives the trip.
fn price(row: &Value, keys: [&str; 2]) -> Option<String> {
    let pricing = row.get("pricing");
    for key in keys {
        let found = row.get(key).or_else(|| pricing?.get(key));
        match found {
            Some(Value::String(text)) if !text.trim().is_empty() => {
                return Some(text.trim().to_owned());
            }
            Some(Value::Number(figure)) => return Some(figure.to_string()),
            _ => {}
        }
    }
    None
}

/// What the row says about thinking, from the first vendor's keys that
/// carry it (`crates/gateway/spec/Endpoint/Models.lean` §8-10).
fn thinking(row: &Value) -> Option<ThinkingStatement> {
    openrouter(row)
        .or_else(|| xai(row))
        .or_else(|| anthropic(row))
        .or_else(|| deepseek(row))
        .or_else(|| moonshot(row))
}

/// `reasoning.{supported_efforts, default_effort, default_enabled}`.
/// A `null` set is every level the gateway takes; a missing one is a
/// switch with no levels.
fn openrouter(row: &Value) -> Option<ThinkingStatement> {
    let said = row.get("reasoning").filter(|said| said.is_object())?;
    let levels = match said.get("supported_efforts") {
        Some(Value::Array(words)) => levels(words),
        Some(Value::Null) => Effort::ALL
            .into_iter()
            .filter(|level| *level != Effort::None)
            .collect(),
        Some(Value::Bool(_) | Value::Number(_) | Value::String(_) | Value::Object(_)) | None => {
            Vec::new()
        }
    };
    Some(ThinkingStatement {
        levels,
        default: said.get("default_effort").and_then(level),
        on: Some(true),
        default_on: said.get("default_enabled").and_then(Value::as_bool),
    })
}

/// `capabilities.{reasoning_effort, default_reasoning_effort}`, or the
/// same two keys at the top of the row.
fn xai(row: &Value) -> Option<ThinkingStatement> {
    let held = [row.get("capabilities"), Some(row)]
        .into_iter()
        .flatten()
        .find(|held| held.get("reasoning_effort").is_some_and(Value::is_array))?;
    Some(ThinkingStatement {
        levels: levels(held.get("reasoning_effort")?.as_array()?),
        default: held.get("default_reasoning_effort").and_then(level),
        on: None,
        default_on: None,
    })
}

/// `capabilities.effort.<level>.supported` and
/// `capabilities.thinking.types.adaptive.supported`.
fn anthropic(row: &Value) -> Option<ThinkingStatement> {
    let capabilities = row.get("capabilities")?;
    let effort = capabilities
        .get("effort")
        .filter(|effort| effort.is_object());
    let adaptive = capabilities
        .pointer("/thinking/types/adaptive/supported")
        .and_then(Value::as_bool);
    if effort.is_none() && adaptive.is_none() {
        return None;
    }
    let supported = |level: &Effort| {
        effort
            .and_then(|effort| effort.get(level.as_str()))
            .and_then(|held| held.get("supported"))
            .and_then(Value::as_bool)
            == Some(true)
    };
    Some(ThinkingStatement {
        levels: Effort::ALL
            .into_iter()
            .filter(|level| *level != Effort::None && supported(level))
            .collect(),
        default: None,
        on: adaptive,
        default_on: None,
    })
}

/// `effort.{supported_levels, default_level}`.
fn deepseek(row: &Value) -> Option<ThinkingStatement> {
    let said = row.get("effort")?;
    Some(ThinkingStatement {
        levels: levels(said.get("supported_levels")?.as_array()?),
        default: said.get("default_level").and_then(level),
        on: None,
        default_on: None,
    })
}

/// `supports_reasoning`: a switch and nothing more.
fn moonshot(row: &Value) -> Option<ThinkingStatement> {
    Some(ThinkingStatement {
        on: Some(row.get("supports_reasoning")?.as_bool()?),
        ..ThinkingStatement::default()
    })
}

/// The levels a list of upstream words names, in the city's ascending
/// order. A word outside the city's seven is not a level, and neither is
/// `none`, because turning thinking off is not one.
fn levels(words: &[Value]) -> Vec<Effort> {
    let named: Vec<Effort> = words.iter().filter_map(level).collect();
    Effort::ALL
        .into_iter()
        .filter(|level| *level != Effort::None && named.contains(level))
        .collect()
}

fn level(word: &Value) -> Option<Effort> {
    let word = word.as_str()?.trim();
    Effort::ALL
        .into_iter()
        .find(|level| level.as_str().eq_ignore_ascii_case(word))
}

/// The upstream's own identity for the model across providers:
/// OpenRouter's `hugging_face_id`, where it is not empty. Its
/// `canonical_slug` is not read: it carries a release date the
/// vendor's own id does not, so it would keep apart two rows the
/// fallback rule of `provider::identity` joins.
fn canonical(row: &Value) -> Option<String> {
    row.get("hugging_face_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
}

/// One row read for everything it states.
///
/// `None` when the row carries no id: a model this city cannot name
/// is a model it cannot call, and inventing a name for it would put
/// an uncallable row in front of a person choosing one.
#[must_use]
pub(crate) fn facts_of(row: &Value) -> Option<ModelFacts> {
    let id = row
        .get("id")
        .and_then(Value::as_str)
        .or_else(|| row.get("name").and_then(Value::as_str))?;
    Some(ModelFacts {
        id: id.to_owned(),
        context_tokens: count(row, &CONTEXT_KEYS),
        max_output_tokens: count(row, &OUTPUT_KEYS).and_then(Ceiling::new),
        input_modalities: modalities(row),
        input_price: price(row, ["input_price", "prompt"]),
        output_price: price(row, ["output_price", "completion"]),
        thinking: thinking(row),
        canonical: canonical(row),
    })
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

    #[test]
    fn a_row_that_states_nothing_but_an_id_states_nothing_but_an_id() {
        let facts = facts_of(&serde_json::json!({ "id": "m-1", "object": "model" }));
        assert_eq!(
            facts,
            Some(ModelFacts {
                id: "m-1".to_owned(),
                ..ModelFacts::default()
            })
        );
    }

    /// The shape a multi-vendor gateway answers with: the two ceilings
    /// under their own names, modalities beside them, prices quoted.
    #[test]
    fn a_row_is_read_for_every_fact_it_carries() {
        let facts = facts_of(&serde_json::json!({
            "id": "vendor/m-2",
            "context_length": 200_000,
            "max_completion_tokens": "64000",
            "input_modalities": ["Text", "image", "text"],
            "pricing": { "prompt": "0.0000004", "completion": "0.0000016" },
        }));
        assert_eq!(
            facts,
            Some(ModelFacts {
                id: "vendor/m-2".to_owned(),
                context_tokens: Some(200_000),
                max_output_tokens: Ceiling::new(64_000),
                input_modalities: vec!["image".to_owned(), "text".to_owned()],
                input_price: Some("0.0000004".to_owned()),
                output_price: Some("0.0000016".to_owned()),
                thinking: None,
                canonical: None,
            })
        );
    }

    /// The other shape: the same two figures one object further down.
    #[test]
    fn a_nested_row_is_read_as_deeply_as_it_states_its_facts() {
        let facts = facts_of(&serde_json::json!({
            "id": "m-3",
            "top_provider": { "context_length": 128_000, "max_completion_tokens": 8_192 },
            "architecture": { "input_modalities": ["text"] },
        }))
        .unwrap();
        assert_eq!(facts.context_tokens, Some(128_000));
        assert_eq!(facts.max_output_tokens, Ceiling::new(8_192));
        assert_eq!(facts.input_modalities, vec!["text".to_owned()]);
    }

    /// Zero is not a ceiling, and a row that states one states none.
    /// The old reading of an unknown ceiling reached a provider as
    /// `max_tokens: 0`, which answers with no content at all.
    #[test]
    fn a_zero_ceiling_is_an_absent_ceiling() {
        let facts = facts_of(&serde_json::json!({ "id": "m-4", "max_output_tokens": 0 })).unwrap();
        assert_eq!(facts.max_output_tokens, None);
    }

    #[test]
    fn a_row_without_a_name_is_no_row() {
        assert_eq!(facts_of(&serde_json::json!({ "object": "model" })), None);
    }

    /// Five model lists read the way a probe reads them, and each row
    /// climbed into the offer the picker is given. OpenRouter's is
    /// rows of its public list; Anthropic's, DeepSeek's and xAI's are
    /// the example answers their API references print; Ollama's is the
    /// shape its OpenAI compatibility page documents for `/v1/models`,
    /// which says nothing about thinking, so every row is unknown.
    #[test]
    fn recorded_listings_are_read_into_offers() {
        use crate::provider::thinking::{EffortSet, OfferSource, Switch, ThinkingOffer};
        let read = |listing: &str| -> Vec<(String, String, ThinkingOffer)> {
            let body: Value = serde_json::from_str(listing).unwrap();
            body["data"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| {
                    let facts = facts_of(row).unwrap();
                    let offer = ThinkingOffer::climb(facts.thinking.as_ref(), None);
                    (
                        facts.id.clone(),
                        crate::provider::identity::canonical(&facts),
                        offer,
                    )
                })
                .collect()
        };
        let said = |levels: &[Effort], on, default, default_on| ThinkingOffer {
            levels: EffortSet::of(levels),
            on,
            default,
            default_on,
            from: OfferSource::Upstream,
            source: None,
        };
        let row = |id: &str, canonical: &str, offer| (id.to_owned(), canonical.to_owned(), offer);
        let unknown = ThinkingOffer::UNKNOWN;
        use Effort::{High, Low, Max, Medium, XHigh};
        assert_eq!(
            [
                read(include_str!("../../tests/listings/openrouter.json")),
                read(include_str!("../../tests/listings/anthropic.json")),
                read(include_str!("../../tests/listings/deepseek.json")),
                read(include_str!("../../tests/listings/xai.json")),
                read(include_str!("../../tests/listings/ollama.json")),
            ],
            [
                vec![
                    row(
                        "deepseek/deepseek-v4.1-flash",
                        "deepseek-v4.1-flash",
                        said(&[Low, High, Max], Switch::Allowed, Some(High), Some(true)),
                    ),
                    row(
                        "inclusionai/ling-3.1-flash",
                        "ling-3.1-flash",
                        said(&[], Switch::Allowed, None, Some(true)),
                    ),
                    row(
                        "stepfun/step-5-preview",
                        "step-5-preview",
                        said(&[Low, Medium, High], Switch::Allowed, Some(Medium), None),
                    ),
                    row(
                        "unbiased/pareto-26.10-preview",
                        "pareto-26.10-preview",
                        unknown
                    ),
                ],
                vec![row(
                    "claude-opus-5",
                    "claude-opus-5",
                    said(
                        &[Low, Medium, High, XHigh, Max],
                        Switch::Allowed,
                        None,
                        None
                    ),
                )],
                vec![
                    row(
                        "deepseek-flash",
                        "deepseek-flash",
                        said(&[Low, High, Max], Switch::Unknown, Some(High), None),
                    ),
                    row(
                        "deepseek-v4-pro",
                        "deepseek-v4-pro",
                        said(&[Low, High, Max], Switch::Unknown, Some(High), None),
                    ),
                ],
                vec![
                    row("latest", "latest", unknown),
                    row(
                        "grok-420-reasoning",
                        "grok-420-reasoning",
                        said(
                            &[Low, Medium, High, XHigh],
                            Switch::Unknown,
                            Some(High),
                            None
                        ),
                    ),
                    row("grok-imagine-image", "grok-imagine-image", unknown),
                ],
                vec![
                    row("qwen3:8b", "qwen3:8b", unknown),
                    row("gpt-oss:20b", "gpt-oss:20b", unknown),
                ],
            ]
        );
    }
}
