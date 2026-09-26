// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which model answers for a tag, and which endpoint stopped answering:
//! the two lines the gateway's endpoint book is rebuilt from besides the
//! attachment itself.

use serde::{Deserialize, Serialize};

use crate::budget::UsdMicros;
use crate::model::{Ceiling, ModelTag};

/// What a model accepts as input.
///
/// A closed judgement rather than a set of flags: every row answers it,
/// and the answer decides whether a picture may be sent at all. The
/// default is the narrow one, because guessing narrow costs a refusal a
/// person can act on and guessing wide costs a 400 from the provider.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum InputKinds {
    #[default]
    Text,
    TextImage,
}

/// `model_selected`: this model answers for this tag, with the facts no
/// probe returns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ModelSelected {
    pub tag: ModelTag,
    /// The attached endpoint the model is called through.
    pub endpoint: String,
    /// The model id as the endpoint spells it.
    pub model: String,
    /// The context window; zero where nobody stated one.
    pub context_tokens: u64,
    /// The output ceiling, written as `null` when none was resolved.
    /// Read it through [`ModelSelected::ceiling`], which owns what a
    /// zero means.
    #[serde(default)]
    pub max_output_tokens: Option<u64>,
    /// Which rung of the ceiling ladder supplied the figure, in the
    /// spelling `gateway::CeilingSource::as_str` owns. Absent when no
    /// rung did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ceiling_from: Option<String>,
    /// Absent on a line written before the key existed, which replays
    /// as text-only rather than as a broken record.
    #[serde(default)]
    pub input: InputKinds,
    pub input_price: UsdMicros,
    pub output_price: UsdMicros,
    pub cache_read_price: UsdMicros,
    pub cache_write_price: UsdMicros,
}

impl ModelSelected {
    /// The registered ceiling, or `None` when the line states none.
    ///
    /// A zero reads as none: a build writing `u64` wrote it when no
    /// catalogue row knew the model, and zero sent as a ceiling is a
    /// request the provider answers with nothing.
    #[must_use]
    pub fn ceiling(&self) -> Option<Ceiling> {
        self.max_output_tokens.and_then(Ceiling::new)
    }
}

/// `endpoint_lost`: the endpoint under this name is gone, and every
/// choice made through it goes with it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EndpointLost {
    pub name: String,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;
    use crate::event::Payload;

    fn selected(max_output_tokens: Option<u64>, ceiling_from: Option<&str>) -> ModelSelected {
        ModelSelected {
            tag: ModelTag::Main,
            endpoint: "house".to_owned(),
            model: "opus-nine".to_owned(),
            context_tokens: 200_000,
            max_output_tokens,
            ceiling_from: ceiling_from.map(str::to_owned),
            input: InputKinds::TextImage,
            input_price: UsdMicros::new(3_000_000),
            output_price: UsdMicros::new(15_000_000),
            cache_read_price: UsdMicros::new(300_000),
            cache_write_price: UsdMicros::new(3_750_000),
        }
    }

    /// The golden bytes: each line as the hand-written writer in
    /// `gateway::router::payload` spelled it, so a ledger already on
    /// disk reads back and a new line is byte-identical to an old one.
    #[test]
    fn each_endpoint_line_keeps_the_bytes_its_ledger_already_holds() {
        let cases: [(Payload, &str); 3] = [
            (
                Payload::of(&selected(Some(32_000), Some("preset"))).unwrap(),
                "{\"cache_read_price\":300000,\"cache_write_price\":3750000,\"ceiling_from\":\"preset\",\"context_tokens\":200000,\"endpoint\":\"house\",\"input\":\"text_image\",\"input_price\":3000000,\"max_output_tokens\":32000,\"model\":\"opus-nine\",\"output_price\":15000000,\"tag\":\"main\"}",
            ),
            (
                Payload::of(&selected(None, None)).unwrap(),
                "{\"cache_read_price\":300000,\"cache_write_price\":3750000,\"context_tokens\":200000,\"endpoint\":\"house\",\"input\":\"text_image\",\"input_price\":3000000,\"max_output_tokens\":null,\"model\":\"opus-nine\",\"output_price\":15000000,\"tag\":\"main\"}",
            ),
            (
                Payload::of(&EndpointLost {
                    name: "house".to_owned(),
                })
                .unwrap(),
                "{\"name\":\"house\"}",
            ),
        ];
        for (payload, wire) in cases {
            assert_eq!(serde_json::to_string(&payload).unwrap(), wire);
            let back: Payload = serde_json::from_str(wire).unwrap();
            assert_eq!(back, payload);
        }
    }

    /// A line from before `input` existed reads as text-only, and its
    /// zero ceiling reads as no ceiling.
    #[test]
    fn an_older_selection_reads_as_text_only_with_no_ceiling() {
        let payload: Payload = serde_json::from_str(
            "{\"cache_read_price\":0,\"cache_write_price\":0,\"context_tokens\":0,\"endpoint\":\"house\",\"input_price\":0,\"max_output_tokens\":0,\"model\":\"m\",\"output_price\":0,\"tag\":\"digest\"}",
        )
        .unwrap();
        let read = payload.read::<ModelSelected>().unwrap();
        assert_eq!(read.input, InputKinds::Text);
        assert_eq!(read.ceiling(), None);
    }
}
