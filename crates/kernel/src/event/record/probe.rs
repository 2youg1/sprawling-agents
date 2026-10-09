// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a probe of a base URL found before anything was attached: how
//! far the call got, and what the model list said.

use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};

use crate::model::{Ceiling, Effort};
use crate::reach::Reach;

/// What a model list said about one model.
///
/// Everything but the id is optional, because for most providers
/// everything but the id is missing. The endpoint book carries these
/// rows into the postcard snapshots of the views and the standing, so
/// `Serialize` is written by hand: an absent `thinking` or `canonical`
/// is omitted only in a human-readable format (kernel D58).
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
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
    #[serde(default)]
    pub thinking: Option<ThinkingStatement>,
    /// The upstream's own identity for this model across providers,
    /// where it states one (`crates/gateway/spec/Provider/Identity.lean`).
    #[serde(default)]
    pub canonical: Option<String>,
}

/// What one model list said about a model's thinking, in the city's
/// spelling of each level. Carried into the snapshots inside
/// [`ModelFacts`], and serialised by hand for the same reason.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ThinkingStatement {
    /// The levels the row named, ascending and without repeats; never
    /// `none`, because turning thinking off is not a level. Empty when
    /// the row stated only a switch.
    #[serde(default)]
    pub levels: Vec<Effort>,
    /// The level the row says it uses when sent none.
    #[serde(default)]
    pub default: Option<Effort>,
    /// Whether the row says the model may be asked to think.
    #[serde(default)]
    pub on: Option<bool>,
    /// Whether the row says the model thinks when sent nothing.
    #[serde(default)]
    pub default_on: Option<bool>,
}

impl Serialize for ModelFacts {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let absence = Absence::of(&serializer);
        let mut row = serializer.serialize_struct("ModelFacts", 8)?;
        row.serialize_field("id", &self.id)?;
        row.serialize_field("context_tokens", &self.context_tokens)?;
        row.serialize_field("max_output_tokens", &self.max_output_tokens)?;
        row.serialize_field("input_modalities", &self.input_modalities)?;
        row.serialize_field("input_price", &self.input_price)?;
        row.serialize_field("output_price", &self.output_price)?;
        absence.option(&mut row, "thinking", &self.thinking)?;
        absence.option(&mut row, "canonical", &self.canonical)?;
        row.end()
    }
}

impl Serialize for ThinkingStatement {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let absence = Absence::of(&serializer);
        let mut statement = serializer.serialize_struct("ThinkingStatement", 4)?;
        absence.list(&mut statement, "levels", &self.levels)?;
        absence.option(&mut statement, "default", &self.default)?;
        absence.option(&mut statement, "on", &self.on)?;
        absence.option(&mut statement, "default_on", &self.default_on)?;
        statement.end()
    }
}

/// How a format carries a key whose value is absent (kernel D58): a
/// human-readable format leaves the key out, which is what every line
/// and CAS blob already holds; postcard is not self-describing and
/// reads fields by position, so a binary format writes every field.
#[derive(Clone, Copy)]
enum Absence {
    Omitted,
    Written,
}

impl Absence {
    fn of<S: Serializer>(serializer: &S) -> Absence {
        if serializer.is_human_readable() {
            Absence::Omitted
        } else {
            Absence::Written
        }
    }

    fn option<S: SerializeStruct, T: Serialize>(
        self,
        fields: &mut S,
        key: &'static str,
        value: &Option<T>,
    ) -> Result<(), S::Error> {
        match (self, value) {
            (Absence::Omitted, None) => fields.skip_field(key),
            (Absence::Omitted, Some(_)) | (Absence::Written, None | Some(_)) => {
                fields.serialize_field(key, value)
            }
        }
    }

    fn list<S: SerializeStruct, T: Serialize>(
        self,
        fields: &mut S,
        key: &'static str,
        value: &[T],
    ) -> Result<(), S::Error> {
        match self {
            Absence::Omitted if value.is_empty() => fields.skip_field(key),
            Absence::Omitted | Absence::Written => fields.serialize_field(key, value),
        }
    }
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

    /// The hand-written `Serialize` leaves a human-readable line exactly
    /// as the derived one with `skip_serializing_if` wrote it (kernel
    /// D58): a row 0.0.10 wrote, a row that stated part of its thinking,
    /// and a row that stated a switch only, with no `null` added.
    #[test]
    fn a_probed_line_with_thinking_facts_keeps_its_bytes() {
        let line = concat!(
            r#"{"base_url":"https://api.example.test/v1","facts":["#,
            r#"{"context_tokens":null,"id":"m-old","input_modalities":[],"#,
            r#""input_price":null,"max_output_tokens":null,"output_price":null},"#,
            r#"{"canonical":"vendor/m-new","context_tokens":200000,"id":"m-new","#,
            r#""input_modalities":["text"],"input_price":null,"max_output_tokens":null,"#,
            r#""output_price":null,"thinking":{"default":"medium","levels":["low","medium","high"]}},"#,
            r#"{"context_tokens":null,"id":"m-switch","input_modalities":[],"#,
            r#""input_price":null,"max_output_tokens":null,"output_price":null,"#,
            r#""thinking":{"on":true}}],"#,
            r#""models":["m-old","m-new","m-switch"],"name":"listed","#,
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
