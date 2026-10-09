// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a probe of a base URL found before anything was attached: how
//! far the call got, and what the model list said.

use serde::{Deserialize, Serialize};

use crate::model::{Ceiling, Effort};
use crate::reach::Reach;

/// What a model list said about one model.
///
/// Everything but the id is optional, because for most providers
/// everything but the id is missing.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
    /// What the row said about thinking levels, read by the gateway's
    /// thinking ladder (`crates/gateway/spec/Provider/Thinking.lean`).
    /// Absent on a row that said nothing and on a line written before
    /// the key existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingStatement>,
    /// The upstream's own identity for this model across providers,
    /// where it states one (`crates/gateway/spec/Provider/Identity.lean`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical: Option<String>,
}

/// What one model list said about a model's thinking, in the city's
/// spelling of each level.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ThinkingStatement {
    /// The levels the row named, ascending and without repeats; never
    /// `none`, because turning thinking off is not a level. Empty when
    /// the row stated only a switch.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub levels: Vec<Effort>,
    /// The level the row says it uses when sent none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Effort>,
    /// Whether the row says the model may be asked to think.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on: Option<bool>,
    /// Whether the row says the model thinks when sent nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_on: Option<bool>,
}

/// `endpoint_probed`: a base URL was asked what it serves, and nothing
/// was attached.
///
/// A probe that could not read a model list still writes this line:
/// `models` and `facts` are then empty rather than absent, and `failed`
/// tells the two apart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EndpointProbed {
    pub name: String,
    pub base_url: String,
    /// How far the call got, stage by stage.
    pub reach: Reach,
    pub models: Vec<String>,
    pub facts: Vec<ModelFacts>,
    /// Why the model list could not be read, when it could not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed: Option<ProbeFailure>,
}

/// The refusal a probe met, as its stable code and subject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProbeFailure {
    pub code: String,
    pub subject: String,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;
    use crate::event::Payload;

    /// The golden bytes the hand-written writer spelled: a probe that
    /// could not read a model list keeps both lists, empty, beside
    /// `failed`.
    #[test]
    fn a_probed_line_keeps_the_bytes_its_ledger_already_holds() {
        let line = concat!(
            r#"{"base_url":"http://127.0.0.1:1","facts":[],"#,
            r#""failed":{"code":"E_PROVIDER","subject":"m"},"models":[],"name":"local","#,
            r#""reach":{"answered":{"detail":"no socket","state":"unreachable"},"#,
            r#""connected":{"state":"refused"},"elapsed_ms":3,"host":"127.0.0.1","#,
            r#""named":{"detail":1,"state":"resolved"},"through":{"state":"direct"}}}"#,
        );
        let read: EndpointProbed = serde_json::from_str::<Payload>(line)
            .unwrap()
            .read()
            .unwrap();
        assert_eq!(
            serde_json::to_string(&Payload::of(&read).unwrap()).unwrap(),
            line
        );
    }
}
