// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};

use crate::model::{Ceiling, Effort};
use crate::reach::Reach;

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ModelFacts {
    pub id: String,

    pub context_tokens: Option<u64>,

    pub max_output_tokens: Option<Ceiling>,

    pub input_modalities: Vec<String>,

    pub input_price: Option<String>,

    pub output_price: Option<String>,

    #[serde(default)]
    pub thinking: Option<ThinkingStatement>,

    #[serde(default)]
    pub generation: Option<crate::GenerationSupport>,
    #[serde(default)]
    pub canonical: Option<String>,

    #[serde(default)]
    pub listing: Listing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Listing {

    #[default]
    Listed,

    Named,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ThinkingStatement {

    #[serde(default)]
    pub levels: Vec<Effort>,

    #[serde(default)]
    pub default: Option<Effort>,

    #[serde(default)]
    pub on: Option<bool>,

    #[serde(default)]
    pub default_on: Option<bool>,
}

impl Serialize for ModelFacts {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let absence = Absence::of(&serializer);
        let mut row = serializer.serialize_struct("ModelFacts", 10)?;
        row.serialize_field("id", &self.id)?;
        row.serialize_field("context_tokens", &self.context_tokens)?;
        row.serialize_field("max_output_tokens", &self.max_output_tokens)?;
        row.serialize_field("input_modalities", &self.input_modalities)?;
        row.serialize_field("input_price", &self.input_price)?;
        row.serialize_field("output_price", &self.output_price)?;
        absence.option(&mut row, "thinking", &self.thinking)?;
        absence.option(&mut row, "generation", &self.generation)?;
        absence.option(&mut row, "canonical", &self.canonical)?;
        absence.listing(&mut row, self.listing)?;
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

    fn listing<S: SerializeStruct>(self, fields: &mut S, value: Listing) -> Result<(), S::Error> {
        match (self, value) {
            (Absence::Omitted, Listing::Listed) => fields.skip_field("listing"),
            (Absence::Omitted, Listing::Named)
            | (Absence::Written, Listing::Listed | Listing::Named) => {
                fields.serialize_field("listing", &value)
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EndpointProbed {
    pub name: String,
    pub base_url: String,

    pub reach: Reach,
    pub models: Vec<String>,
    pub facts: Vec<ModelFacts>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed: Option<ProbeFailure>,
}

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
