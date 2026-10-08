// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a person attached, which model answers for a tag, and which
//! endpoint stopped answering: the three lines the gateway's endpoint
//! book is rebuilt from.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::budget::UsdMicros;
use crate::locator::B3Hash;
use crate::model::{Ceiling, DialectKind, ModelTag};
use crate::reach::Proxying;
use crate::secret::SecretRef;

/// `endpoint_attached`: a base URL, its dialect, the credential's vault
/// place and the model ids the endpoint reported.
///
/// **The credential is a reference, never the key**: that is what keeps
/// this line exportable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EndpointAttached {
    pub name: String,
    pub base_url: String,
    pub dialect: DialectKind,
    /// Absent for an endpoint that takes no credential.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub auth: Option<SecretRef>,
    /// The header the credential travels in, when it is not a bearer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_header: Option<String>,
    pub models: Vec<String>,
    /// `gateway::ConnectionKind` in its own spelling. Absent on a line
    /// written by a build with one registration per dialect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_kind: Option<String>,
    /// Absent means true: every line written before the key existed
    /// came from a probe that succeeded.
    #[serde(default = "probed_by_default")]
    pub probed: bool,
    /// What the person settled; absent when they settled nothing, and
    /// read as nothing settled when it is not an object.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "readable_tuning"
    )]
    pub tuning: Option<AttachedTuning>,
    /// The model facts read at attach or re-probe time, as one CAS blob
    /// (`crates/kernel/spec/Event/Record.lean` §8-88, kernel D57). Absent on a line
    /// written before the key existed and on an attach that read no list;
    /// a replay then rebuilds from `models` alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub facts_blob: Option<B3Hash>,
}

/// How the person set an endpoint up, as the line keeps it.
///
/// Tolerant in one direction only: a key that is missing reads as
/// nothing settled, which is what every line written before the key
/// existed means, and a key that is present and unreadable is left out
/// rather than guessed at, because a deadline this city invented would
/// be a deadline nobody can explain.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AttachedTuning {
    /// Explicit ordered accounts; absent keeps the legacy registration.
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
    /// Absent for the default, which is how the line says nothing was
    /// settled here too.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "readable"
    )]
    pub proxying: Option<Proxying>,
    /// How many calls may be in flight at once; absent when nobody
    /// settled it, and a line written before the key existed reads so.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "readable"
    )]
    pub max_in_flight: Option<u32>,
    /// How many more times one account is asked before the next account
    /// takes the request; absent when nobody settled it.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "readable"
    )]
    pub account_retries: Option<crate::account_recovery::AccountRetries>,
    /// Header name and the person's own spelling of its value, which for
    /// a credential is its reference.
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

/// A value this build can read, or `None`: the tolerance
/// [`AttachedTuning`] documents, decided in one place.
fn readable<'de, D: Deserializer<'de>, T: DeserializeOwned>(
    held: D,
) -> Result<Option<T>, D::Error> {
    Ok(serde_json::from_value(Value::deserialize(held)?).ok())
}

/// The rows that are two strings; a row that is not is left out rather
/// than half read.
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

    /// A tuning key this build cannot read is left out, and the rest of
    /// the line still reads.
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

    /// A registered ceiling is the one the line states.
    #[test]
    fn a_stated_ceiling_reads_back_as_that_ceiling() {
        assert_eq!(
            selected(Some(8192), Some("catalogue")).ceiling(),
            Ceiling::new(8192)
        );
        assert_eq!(selected(None, None).ceiling(), None);
    }
}
