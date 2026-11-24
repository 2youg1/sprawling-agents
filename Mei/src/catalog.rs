// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde_json::Value;

use crate::error::{Code, Diagnostic, Error, Result, protocol};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub model: String,
    pub context_tokens: Option<u64>,
    pub efforts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    pub model: String,
    pub effort: Option<String>,
}

fn invalid_catalog(subject: impl Into<String>, value: &Value) -> Error {
    Diagnostic::new(
        Code::Config,
        "select a model from the Magpie catalog",
        subject,
        "expose at least one chat model in Magpie, or set HARNESS_MODEL explicitly",
    )
    .with_path("response.data")
    .with_value(value.to_string())
    .into()
}

fn model(entry: &Value) -> Option<Selection> {
    let id = entry.get("id")?.as_str()?.trim();
    if id.is_empty() {
        return None;
    }
    let context_tokens = entry
        .get("context_window")
        .and_then(Value::as_u64)
        .filter(|tokens| *tokens > 0);
    let efforts = entry
        .get("reasoning_levels")
        .and_then(Value::as_array)
        .map(|levels| {
            levels
                .iter()
                .filter_map(Value::as_str)
                .filter(|level| !level.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    Some(Selection {
        model: id.to_owned(),
        context_tokens,
        efforts,
    })
}

pub fn select(catalog: &Value) -> Result<Selection> {
    select_for(catalog, None)
}

pub fn select_for(catalog: &Value, requested: Option<&str>) -> Result<Selection> {
    let entries = catalog
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid_catalog("data is missing or is not an array", catalog))?;
    let preferred = catalog.get("first_id").and_then(Value::as_str);
    if let Some(id) = requested {
        return entries
            .iter()
            .find(|entry| entry.get("id").and_then(Value::as_str) == Some(id))
            .and_then(model)
            .ok_or_else(|| invalid_catalog(format!("the catalog has no entry for {id}"), catalog));
    }
    let entry = preferred
        .and_then(|id| {
            entries
                .iter()
                .find(|entry| entry.get("id").and_then(Value::as_str) == Some(id))
        })
        .or_else(|| entries.first())
        .ok_or_else(|| invalid_catalog("the catalog has no visible chat model", catalog))?;
    model(entry)
        .ok_or_else(|| invalid_catalog("the selected entry has no non-empty string id", entry))
}

pub fn route(answer: &Value) -> Result<Route> {
    let route = answer.get("route").ok_or_else(|| {
        protocol(
            "read the Magpie route observation",
            "route is missing",
            "return the completed route for the supplied X-Magpie-Session",
        )
        .with_path("response.route")
    })?;
    let model = route
        .get("model")
        .and_then(Value::as_str)
        .filter(|model| !model.is_empty())
        .ok_or_else(|| {
            protocol(
                "read the Magpie route observation",
                "route.model is missing or empty",
                "return the concrete provider/model selected by the gateway",
            )
            .with_path("response.route.model")
            .with_value(route.to_string())
        })?;
    let effort = route
        .get("effort")
        .and_then(Value::as_str)
        .filter(|effort| !effort.is_empty())
        .map(str::to_owned);
    Ok(Route {
        model: model.to_owned(),
        effort,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn published_model_identifiers_round_trip_through_the_catalog() {
        let catalog = serde_json::json!({
            "first_id": " model/a ",
            "data": [{"id": "other"}, {"id": " model/a "}]
        });
        let discovered = select(&catalog).unwrap();
        assert_eq!(discovered.model, "model/a");
        assert_eq!(
            select_for(&catalog, Some(&discovered.model)).unwrap(),
            discovered
        );
    }

    #[test]
    fn the_preferred_route_carries_context_and_efforts() {
        let catalog = serde_json::json!({
            "first_id": "group/plan-first",
            "data": [
                {"id": "provider/other", "context_window": 200000, "reasoning_levels": ["low"]},
                {"id": "group/plan-first", "context_window": 1048576, "reasoning_levels": ["none", "low", "high", "max"]}
            ]
        });
        assert_eq!(
            select(&catalog).unwrap(),
            Selection {
                model: "group/plan-first".to_owned(),
                context_tokens: Some(1048576),
                efforts: ["none", "low", "high", "max"].map(str::to_owned).to_vec(),
            }
        );
    }

    #[test]
    fn a_stale_preference_falls_back_to_the_first_visible_entry() {
        let catalog = serde_json::json!({
            "first_id": "group/removed",
            "data": [{"id": "provider/first"}]
        });
        assert_eq!(select(&catalog).unwrap().model, "provider/first");
    }

    #[test]
    fn an_empty_catalog_cannot_invent_a_model() {
        let error = select(&serde_json::json!({"data": []})).unwrap_err();
        assert!(error.to_string().contains("no visible chat model"));
    }

    #[test]
    fn a_completed_route_distinguishes_actual_and_unstated_effort() {
        let answer = serde_json::json!({"route": {"model": "kimi-code-cn/kimi-for-coding", "effort": "high"}});
        assert_eq!(
            route(&answer).unwrap(),
            Route {
                model: "kimi-code-cn/kimi-for-coding".to_owned(),
                effort: Some("high".to_owned()),
            }
        );
        let answer = serde_json::json!({"route": {"model": "provider/model", "effort": null}});
        assert_eq!(route(&answer).unwrap().effort, None);
    }
}
