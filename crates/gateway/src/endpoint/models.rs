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

use kernel::Ceiling;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// What a model list said about one model.
///
/// Everything but the id is optional, because for most providers
/// everything but the id is missing.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ModelFacts {
    pub id: String,
    /// How many tokens the model reads in one call.
    pub context_tokens: Option<u64>,
    /// How many tokens it may write back.
    pub max_output_tokens: Option<Ceiling>,
    /// What it accepts, in the provider's own words (`text`, `image`,
    /// `audio`). Empty means the row said nothing, never "text only".
    pub input_modalities: Vec<String>,
    /// The provider's own input price, verbatim, unit included when the
    /// provider stated one.
    pub input_price: Option<String>,
    /// The provider's own output price, verbatim.
    pub output_price: Option<String>,
}

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
        _ => None,
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

impl ModelFacts {
    /// One row read for everything it states.
    ///
    /// `None` when the row carries no id: a model this city cannot name
    /// is a model it cannot call, and inventing a name for it would put
    /// an uncallable row in front of a person choosing one.
    #[must_use]
    pub fn read(row: &Value) -> Option<ModelFacts> {
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
        })
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

    #[test]
    fn a_row_that_states_nothing_but_an_id_states_nothing_but_an_id() {
        let facts = ModelFacts::read(&serde_json::json!({ "id": "m-1", "object": "model" }));
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
        let facts = ModelFacts::read(&serde_json::json!({
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
            })
        );
    }

    /// The other shape: the same two figures one object further down.
    #[test]
    fn a_nested_row_is_read_as_deeply_as_it_states_its_facts() {
        let facts = ModelFacts::read(&serde_json::json!({
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
        let facts =
            ModelFacts::read(&serde_json::json!({ "id": "m-4", "max_output_tokens": 0 })).unwrap();
        assert_eq!(facts.max_output_tokens, None);
    }

    #[test]
    fn a_row_without_a_name_is_no_row() {
        assert_eq!(
            ModelFacts::read(&serde_json::json!({ "object": "model" })),
            None
        );
    }
}
