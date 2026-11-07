// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Sampling support and the generation a session freezes. The person's
//! corrections outrank the upstream model list, which outranks presets;
//! no provider error is turned into a capability statement. Specified by
//! `crates/gateway/spec/Provider/Generation.lean` §8-41, gateway D64.

use kernel::{
    AxCode, AxError, Ceiling, FrozenGeneration, GenerationSettings, ModelGeneration,
    ParameterSource, ParameterSupport,
};
use serde_json::Value;

use crate::market::ModelEntry;
use crate::router::AttachedEndpoint;

/// One parameter's support and the rung that stated it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParameterAnswer {
    pub support: ParameterSupport,
    pub source: Option<ParameterSource>,
}

/// The model editor's resolved view of one model's generation settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationAnswer {
    pub settings: GenerationSettings,
    pub output_tokens: Option<Ceiling>,
    pub temperature: ParameterAnswer,
    pub top_p: ParameterAnswer,
    pub top_k: ParameterAnswer,
    /// The raw override pointer that controls this face's output ceiling.
    pub output_pointer: &'static str,
}

fn person<'a>(endpoint: &'a AttachedEndpoint, model: &str) -> Option<&'a ModelGeneration> {
    endpoint
        .tuning
        .generation
        .iter()
        .find(|row| row.model == model)
}

fn stated(
    person: Option<ParameterSupport>,
    upstream: Option<ParameterSupport>,
    preset: Option<ParameterSupport>,
) -> ParameterAnswer {
    match (person, upstream, preset) {
        (Some(support), _, _) => ParameterAnswer {
            support,
            source: Some(ParameterSource::Person),
        },
        (None, Some(support), _) => ParameterAnswer {
            support,
            source: Some(ParameterSource::Upstream),
        },
        (None, None, Some(support)) => ParameterAnswer {
            support,
            source: Some(ParameterSource::Preset),
        },
        (None, None, None) => ParameterAnswer {
            support: ParameterSupport::Unknown,
            source: None,
        },
    }
}

/// The defaults and support answers for one `(Endpoint, model, dialect)`.
#[must_use]
pub fn answer_for(endpoint: &AttachedEndpoint, model: &str) -> GenerationAnswer {
    let person = person(endpoint, model);
    let upstream = endpoint
        .models
        .iter()
        .find(|row| row.id == model)
        .and_then(|row| row.generation)
        .unwrap_or_default();
    let preset = super::preset::generation_for(&endpoint.base_url, endpoint.dialect);
    let own = person.map(|row| row.support).unwrap_or_default();
    GenerationAnswer {
        settings: person.map(|row| row.settings.clone()).unwrap_or_default(),
        output_tokens: person.and_then(|row| row.output_tokens),
        temperature: stated(own.temperature, upstream.temperature, preset.temperature),
        top_p: stated(own.top_p, upstream.top_p, preset.top_p),
        top_k: stated(own.top_k, upstream.top_k, preset.top_k),
        output_pointer: output_pointer(endpoint),
    }
}

fn invalid(model: &str, parameter: &str, expected: &str) -> AxError {
    AxError::failure(
        AxCode::ConfigInvalid,
        "resolve model generation",
        format!("{model}: {parameter} must be {expected}"),
    )
    .with_recovery(format!("enter {expected}, or clear {parameter}"))
}

fn number(model: &str, parameter: &str, text: &str) -> Result<Value, AxError> {
    match serde_json::from_str(text).map_err(|_| invalid(model, parameter, "a JSON number"))? {
        Value::Number(value) => Ok(Value::Number(value)),
        Value::Null | Value::Bool(_) | Value::String(_) | Value::Array(_) | Value::Object(_) => {
            Err(invalid(model, parameter, "a JSON number"))
        }
    }
}

fn top_k(model: &str, text: &str) -> Result<Value, AxError> {
    match number(model, "top_k", text)? {
        Value::Number(value) if value.as_u64().is_some() => Ok(Value::Number(value)),
        Value::Number(_) => Err(invalid(model, "top_k", "a non-negative integer")),
        Value::Null | Value::Bool(_) | Value::String(_) | Value::Array(_) | Value::Object(_) => {
            Err(invalid(model, "top_k", "a non-negative integer"))
        }
    }
}

/// Parse the typed sampling values for a request body. Raw overrides are
/// deliberately not read here: they keep their existing JSON-or-text
/// semantics and are applied after these fields.
pub(crate) fn request_values(
    model: &str,
    settings: &GenerationSettings,
) -> Result<Vec<(&'static str, Value)>, AxError> {
    let mut values = Vec::new();
    if let Some(text) = &settings.temperature {
        values.push(("temperature", number(model, "temperature", text)?));
    }
    if let Some(text) = &settings.top_p {
        values.push(("top_p", number(model, "top_p", text)?));
    }
    if let Some(text) = &settings.top_k {
        values.push(("top_k", top_k(model, text)?));
    }
    Ok(values)
}

fn supported(
    model: &str,
    parameter: &str,
    answer: ParameterAnswer,
    set: bool,
) -> Result<(), AxError> {
    if set && answer.support == ParameterSupport::Unsupported {
        return Err(AxError::failure(
            AxCode::ConfigInvalid,
            "resolve model generation",
            format!("{model}: {parameter} is marked unsupported"),
        )
        .with_recovery(format!(
            "clear {parameter}, or correct its support statement in the model editor"
        )));
    }
    Ok(())
}

fn output_pointer(endpoint: &AttachedEndpoint) -> &'static str {
    match endpoint.dialect {
        kernel::DialectKind::Anthropic => "/max_tokens",
        kernel::DialectKind::OpenAiResponses => "/max_output_tokens",
        kernel::DialectKind::OpenAi => {
            match super::preset::chat_spelling(&endpoint.base_url).ceiling {
                super::preset::CeilingField::MaxTokens => "/max_tokens",
                super::preset::CeilingField::MaxCompletionTokens => "/max_completion_tokens",
            }
        }
    }
}

fn check_output(
    model: &str,
    preferred: Option<Ceiling>,
    hard: Option<Ceiling>,
) -> Result<(), AxError> {
    if let (Some(wanted), Some(ceiling)) = (preferred, hard)
        && wanted > ceiling
    {
        return Err(AxError::failure(
            AxCode::ConfigInvalid,
            "resolve model generation",
            format!(
                "{model}: output preference {wanted} exceeds the model's stated ceiling {ceiling}"
            ),
        )
        .with_recovery(
            "lower the session output preference, or correct the model's hard ceiling",
        ));
    }
    Ok(())
}

fn effective_output(
    endpoint: &AttachedEndpoint,
    model: &ModelEntry,
    preferred: Option<Ceiling>,
) -> Result<Option<Ceiling>, AxError> {
    let hard = model
        .max_output_tokens
        .filter(|_| model.ceiling_from != Some(crate::FigureSource::Policy));
    check_output(&model.id, preferred, hard)?;
    let mut effective = preferred.or(model.max_output_tokens);
    if let Some((_, text)) = endpoint
        .tuning
        .overrides
        .iter()
        .rfind(|(pointer, _)| pointer == output_pointer(endpoint))
    {
        let parsed = number(&model.id, "output override", text)?;
        let count = parsed
            .as_u64()
            .and_then(Ceiling::new)
            .ok_or_else(|| invalid(&model.id, "output override", "a positive integer"))?;
        effective = Some(count);
    }
    Ok(effective)
}

/// Resolve and validate the generation a new or existing session uses.
///
/// # Errors
/// Refuses malformed numeric text, a value whose resolved capability is
/// `Unsupported`, an output preference above a stated hard ceiling, and
/// a raw output override that is not a positive integer.
pub fn freeze(
    endpoint: &AttachedEndpoint,
    model: &ModelEntry,
) -> Result<FrozenGeneration, AxError> {
    let answer = answer_for(endpoint, &model.id);
    validate_answer(&model.id, &answer)?;
    Ok(FrozenGeneration {
        settings: answer.settings.clone(),
        max_tokens: effective_output(endpoint, model, answer.output_tokens)?,
        overrides: endpoint.tuning.overrides.clone(),
    })
}

fn validate_answer(model: &str, answer: &GenerationAnswer) -> Result<(), AxError> {
    request_values(model, &answer.settings)?;
    supported(
        model,
        "temperature",
        answer.temperature,
        answer.settings.temperature.is_some(),
    )?;
    supported(
        model,
        "top_p",
        answer.top_p,
        answer.settings.top_p.is_some(),
    )?;
    supported(
        model,
        "top_k",
        answer.top_k,
        answer.settings.top_k.is_some(),
    )
}

/// Validate one complete generation draft before an endpoint attachment
/// replaces the book. No row is written when any row is invalid.
///
/// # Errors
/// Refuses duplicate or unregistered model ids, malformed values,
/// unsupported values and a preference above a stated hard ceiling.
pub fn validate(endpoint: &AttachedEndpoint) -> Result<(), AxError> {
    if endpoint.tuning.generation.is_empty() {
        return Ok(());
    }
    let catalogue = crate::market::MarketSnapshot::builtin()?;
    let mut seen = std::collections::BTreeSet::new();
    for row in &endpoint.tuning.generation {
        if !seen.insert(row.model.as_str()) {
            return Err(invalid(&row.model, "model", "listed only once"));
        }
        let listed = endpoint
            .models
            .iter()
            .find(|facts| facts.id == row.model)
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::ConfigInvalid,
                    "save model generation",
                    format!("{} is not registered at {}", row.model, endpoint.name),
                )
                .with_recovery("keep the model registered, or remove its generation draft")
            })?;
        let answer = answer_for(endpoint, &row.model);
        validate_answer(&row.model, &answer)?;
        let hard = listed
            .max_output_tokens
            .or_else(|| super::preset::ceiling_for(&endpoint.base_url, &row.model))
            .or_else(|| {
                catalogue
                    .lookup(&row.model)
                    .and_then(|entry| entry.max_output_tokens)
            });
        check_output(&row.model, answer.output_tokens, hard)?;
    }
    Ok(())
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
    use crate::endpoint::AuthSpec;
    use crate::provider::registry::ConnectionKind;
    use crate::router::EndpointTuning;
    use kernel::GenerationSupport;
    use kernel::event::record::ModelFacts;

    fn endpoint(rows: Vec<ModelGeneration>, facts: Vec<ModelFacts>) -> AttachedEndpoint {
        AttachedEndpoint {
            name: "house".to_owned(),
            base_url: "https://api.openai.com/v1".to_owned(),
            dialect: kernel::DialectKind::OpenAi,
            connection_kind: ConnectionKind::OpenAiCompat,
            auth: AuthSpec::None,
            models: facts,
            probed: true,
            tuning: EndpointTuning {
                generation: rows,
                ..EndpointTuning::default()
            },
            facts_blob: None,
        }
    }

    fn generation(support: ParameterSupport) -> ModelGeneration {
        ModelGeneration {
            model: "m".to_owned(),
            settings: GenerationSettings {
                temperature: Some("0.2".to_owned()),
                ..GenerationSettings::default()
            },
            output_tokens: Ceiling::new(4096),
            support: GenerationSupport {
                temperature: Some(support),
                ..GenerationSupport::default()
            },
        }
    }

    #[test]
    fn the_persons_correction_outranks_upstream_and_preset() {
        let answer = answer_for(
            &endpoint(
                vec![generation(ParameterSupport::Supported)],
                vec![ModelFacts {
                    id: "m".to_owned(),
                    generation: Some(GenerationSupport {
                        temperature: Some(ParameterSupport::Unsupported),
                        ..GenerationSupport::default()
                    }),
                    ..ModelFacts::default()
                }],
            ),
            "m",
        );
        assert_eq!(
            answer.temperature,
            ParameterAnswer {
                support: ParameterSupport::Supported,
                source: Some(ParameterSource::Person)
            }
        );
    }

    #[test]
    fn an_output_preference_above_a_known_hard_ceiling_is_refused() {
        let model = ModelEntry {
            id: "m".to_owned(),
            max_output_tokens: Ceiling::new(1024),
            ..crate::market::MarketSnapshot::builtin()
                .unwrap()
                .lookup("local")
                .unwrap()
                .clone()
        };
        assert!(
            freeze(
                &endpoint(vec![generation(ParameterSupport::Supported)], vec![]),
                &model
            )
            .is_err()
        );
    }

    #[test]
    fn a_raw_output_override_is_the_effective_frozen_ceiling() {
        let mut attached = endpoint(Vec::new(), Vec::new());
        attached.tuning.overrides = vec![("/max_completion_tokens".to_owned(), "2048".to_owned())];
        let model = ModelEntry {
            id: "m".to_owned(),
            ..crate::market::MarketSnapshot::builtin()
                .unwrap()
                .lookup("local")
                .unwrap()
                .clone()
        };
        assert_eq!(
            freeze(&attached, &model).unwrap().max_tokens,
            Ceiling::new(2048)
        );
    }

    #[test]
    fn a_known_unsupported_parameter_is_refused_until_the_person_corrects_it() {
        let mut row = generation(ParameterSupport::Supported);
        row.settings.top_k = Some("40".to_owned());
        let mut attached = endpoint(
            vec![row],
            vec![ModelFacts {
                id: "m".to_owned(),
                ..ModelFacts::default()
            }],
        );
        assert!(validate(&attached).is_err());
        attached.tuning.generation[0].support.top_k = Some(ParameterSupport::Supported);
        assert!(validate(&attached).is_ok());
    }

    proptest::proptest! {
        /// Every combination of the three support rungs agrees with the
        /// precedence proved in `spec/Provider/Generation.lean`.
        #[test]
        fn the_support_ladder_keeps_the_lean_properties(
            person in proptest::option::of(proptest::sample::select(vec![
                ParameterSupport::Unknown,
                ParameterSupport::Supported,
                ParameterSupport::Unsupported,
            ])),
            upstream in proptest::option::of(proptest::sample::select(vec![
                ParameterSupport::Unknown,
                ParameterSupport::Supported,
                ParameterSupport::Unsupported,
            ])),
            preset in proptest::option::of(proptest::sample::select(vec![
                ParameterSupport::Unknown,
                ParameterSupport::Supported,
                ParameterSupport::Unsupported,
            ])),
        ) {
            let answer = stated(person, upstream, preset);
            let expected = person.or(upstream).or(preset);
            proptest::prop_assert_eq!(answer.support, expected.unwrap_or(ParameterSupport::Unknown));
            proptest::prop_assert_eq!(answer.source.is_some(), expected.is_some());
            if person.is_some() {
                proptest::prop_assert_eq!(answer.source, Some(ParameterSource::Person));
            } else if upstream.is_some() {
                proptest::prop_assert_eq!(answer.source, Some(ParameterSource::Upstream));
            } else if preset.is_some() {
                proptest::prop_assert_eq!(answer.source, Some(ParameterSource::Preset));
            }
        }
    }

    #[test]
    fn typed_numbers_reach_the_request_and_top_k_must_be_an_integer() {
        let settings = GenerationSettings {
            temperature: Some("0.2".to_owned()),
            top_p: Some("0.9".to_owned()),
            top_k: Some("40".to_owned()),
        };
        assert_eq!(
            request_values("m", &settings).unwrap(),
            vec![
                ("temperature", serde_json::json!(0.2)),
                ("top_p", serde_json::json!(0.9)),
                ("top_k", serde_json::json!(40)),
            ]
        );
        assert!(
            request_values(
                "m",
                &GenerationSettings {
                    top_k: Some("0.5".to_owned()),
                    ..GenerationSettings::default()
                }
            )
            .is_err()
        );
    }
}
