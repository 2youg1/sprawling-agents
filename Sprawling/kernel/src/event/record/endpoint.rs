// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::budget::UsdMicros;
use crate::locator::B3Hash;
use crate::model::{Ceiling, DialectKind, ModelTag};
use crate::reach::Proxying;
use crate::secret::SecretRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EndpointAttached {
    pub name: String,
    pub base_url: String,
    pub dialect: DialectKind,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub auth: Option<SecretRef>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_header: Option<String>,
    pub models: Vec<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_kind: Option<String>,

    #[serde(default = "probed_by_default")]
    pub probed: bool,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "readable_tuning"
    )]
    pub tuning: Option<AttachedTuning>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub facts_blob: Option<B3Hash>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AttachedTuning {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accounts: Option<Vec<super::ProviderAccount>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "readable"
    )]
    pub label: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "readable"
    )]
    pub timeout_ms: Option<u64>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "readable"
    )]
    pub stream_idle_timeout_ms: Option<u64>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "readable"
    )]
    pub request_max_retries: Option<u32>,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "readable"
    )]
    pub proxying: Option<Proxying>,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "readable"
    )]
    pub max_in_flight: Option<u32>,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "readable"
    )]
    pub account_retries: Option<crate::account_recovery::AccountRetries>,

    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "readable_pairs"
    )]
    pub extra_headers: Vec<(String, String)>,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "readable_pairs"
    )]
    pub overrides: Vec<(String, String)>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub generation: Vec<crate::ModelGeneration>,
}

fn readable_tuning<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<AttachedTuning>, D::Error> {
    let value = Value::deserialize(deserializer)?;
    if !value.is_object() {
        return Ok(None);
    }
    AttachedTuning::deserialize(value)
        .map(Some)
        .map_err(serde::de::Error::custom)
}

const fn probed_by_default() -> bool {
    true
}

fn readable<'de, D: Deserializer<'de>, T: DeserializeOwned>(
    held: D,
) -> Result<Option<T>, D::Error> {
    Ok(serde_json::from_value(Value::deserialize(held)?).ok())
}

fn readable_pairs<'de, D: Deserializer<'de>>(held: D) -> Result<Vec<(String, String)>, D::Error> {
    let Value::Array(rows) = Value::deserialize(held)? else {
        return Ok(Vec::new());
    };
    Ok(rows
        .into_iter()
        .filter_map(|row| match row {
            Value::Array(cells) => match (cells.first(), cells.get(1)) {
                (Some(Value::String(name)), Some(Value::String(value))) => {
                    Some((name.clone(), value.clone()))
                }
                _ => None,
            },
            Value::Null
            | Value::Bool(_)
            | Value::Number(_)
            | Value::String(_)
            | Value::Object(_) => None,
        })
        .collect())
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum InputKinds {
    #[default]
    Text,
    TextImage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ModelSelected {
    pub tag: ModelTag,

    pub endpoint: String,

    pub model: String,

    pub context_tokens: u64,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_from: Option<String>,

    #[serde(default)]
    pub max_output_tokens: Option<u64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ceiling_from: Option<String>,

    #[serde(default)]
    pub input: InputKinds,
    pub input_price: UsdMicros,
    pub output_price: UsdMicros,
    pub cache_read_price: UsdMicros,
    pub cache_write_price: UsdMicros,
}

impl ModelSelected {
    #[must_use]
    pub fn ceiling(&self) -> Option<Ceiling> {
        self.max_output_tokens.and_then(Ceiling::new)
    }
}

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
            window_from: None,
            max_output_tokens,
            ceiling_from: ceiling_from.map(str::to_owned),
            input: InputKinds::TextImage,
            input_price: UsdMicros::new(3_000_000),
            output_price: UsdMicros::new(15_000_000),
            cache_read_price: UsdMicros::new(300_000),
            cache_write_price: UsdMicros::new(3_750_000),
        }
    }

    #[test]
    fn each_endpoint_line_keeps_the_bytes_its_ledger_already_holds() {
        let cases: [(Payload, &str); 5] = [
            (
                Payload::of(&EndpointAttached {
                    name: "house".to_owned(),
                    base_url: "https://api.example.test/v1".to_owned(),
                    dialect: DialectKind::OpenAi,
                    auth: Some(SecretRef::parse("secret:house/key").unwrap()),
                    auth_header: Some("x-api-key".to_owned()),
                    models: vec!["opus-nine".to_owned()],
                    connection_kind: Some("openai_compat".to_owned()),
                    probed: false,
                    tuning: Some(AttachedTuning {
                        label: Some("House".to_owned()),
                        timeout_ms: Some(30_000),
                        request_max_retries: Some(2),
                        proxying: Some(Proxying::Never),
                        extra_headers: vec![("x-team".to_owned(), "secret:house/team".to_owned())],
                        ..AttachedTuning::default()
                    }),
                    facts_blob: None,
                })
                .unwrap(),
                "{\"auth\":\"secret:house/key\",\"auth_header\":\"x-api-key\",\"base_url\":\"https://api.example.test/v1\",\"connection_kind\":\"openai_compat\",\"dialect\":\"open_ai\",\"models\":[\"opus-nine\"],\"name\":\"house\",\"probed\":false,\"tuning\":{\"extra_headers\":[[\"x-team\",\"secret:house/team\"]],\"label\":\"House\",\"proxying\":\"never\",\"request_max_retries\":2,\"timeout_ms\":30000}}",
            ),
            (
                Payload::of(&EndpointAttached {
                    name: "local".to_owned(),
                    base_url: "http://127.0.0.1:11434/v1".to_owned(),
                    dialect: DialectKind::OpenAi,
                    auth: None,
                    auth_header: None,
                    models: Vec::new(),
                    connection_kind: Some("openai_compat".to_owned()),
                    probed: true,
                    tuning: None,
                    facts_blob: None,
                })
                .unwrap(),
                "{\"base_url\":\"http://127.0.0.1:11434/v1\",\"connection_kind\":\"openai_compat\",\"dialect\":\"open_ai\",\"models\":[],\"name\":\"local\",\"probed\":true}",
            ),
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

    #[test]
    fn an_unreadable_tuning_key_is_left_out_rather_than_refusing_the_line() {
        let payload: Payload = serde_json::from_str(
            "{\"base_url\":\"u\",\"dialect\":\"anthropic\",\"models\":[],\"name\":\"n\",\"tuning\":{\"timeout_ms\":-1,\"label\":7,\"overrides\":[[\"a\",\"b\"],[\"c\"],3]}}",
        )
        .unwrap();
        let read = payload.read::<EndpointAttached>().unwrap();
        assert!(read.probed);
        assert_eq!(
            read.tuning,
            Some(AttachedTuning {
                overrides: vec![("a".to_owned(), "b".to_owned())],
                ..AttachedTuning::default()
            })
        );
    }

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

    #[test]
    fn a_stated_ceiling_reads_back_as_that_ceiling() {
        assert_eq!(
            selected(Some(8192), Some("catalogue")).ceiling(),
            Ceiling::new(8192)
        );
        assert_eq!(selected(None, None).ceiling(), None);
    }
}
