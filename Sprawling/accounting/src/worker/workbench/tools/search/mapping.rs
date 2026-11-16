// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What `web_search` asks of the model and hands to one supplier: the
//! parameters it offers, derived from the supplier frozen with the run,
//! and the judgement of that supplier's mapping against the remote
//! tool's `inputSchema` (`crates/accounting/spec/Connectors.lean` §8-35).

use kernel::config::SearchSupplier;
use kernel::{AxCode, AxError, Payload};
use serde_json::{Map, Value, json};

/// The three names the model calls `web_search` with.
const QUERY: &str = "query";
const OBJECTIVE: &str = "objective";
const COUNT: &str = "num_results";

/// What one call asked for, read and judged against the parameters the
/// supplier was offered with.
pub(super) struct Asked {
    query: String,
    objective: Option<String>,
    count: Option<u64>,
}

/// The parameters `web_search` offers under `supplier`: `query` always,
/// `objective` only where the supplier maps one (and then required),
/// `num_results` only where it maps a count. The model is never offered
/// a parameter this supplier cannot take.
pub(super) fn parameters(supplier: &SearchSupplier) -> Map<String, Value> {
    let text = json!({"type": "string", "minLength": 1});
    let mut properties = Map::new();
    let mut required = vec![Value::from(QUERY)];
    properties.insert(QUERY.to_owned(), text.clone());
    if supplier.objective_field.is_some() {
        properties.insert(OBJECTIVE.to_owned(), text);
        required.push(Value::from(OBJECTIVE));
    }
    if supplier.count_field.is_some() {
        properties.insert(COUNT.to_owned(), json!({"type": "integer", "minimum": 1}));
    }
    let mut schema = Map::new();
    schema.insert("type".to_owned(), Value::from("object"));
    schema.insert("properties".to_owned(), Value::Object(properties));
    schema.insert("required".to_owned(), Value::Array(required));
    schema.insert("additionalProperties".to_owned(), Value::Bool(false));
    schema
}

impl Asked {
    /// Reads one call's arguments by the parameters `supplier` offered.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` for a name the parameters do not offer, a missing
    /// or empty `query` or required `objective`, and a `num_results`
    /// that is not a positive integer.
    pub(super) fn read(supplier: &SearchSupplier, args: &Payload) -> Result<Asked, AxError> {
        let args = args.as_map();
        let offered = parameters(supplier);
        let offered = offered.get("properties").and_then(Value::as_object);
        if let Some(name) = args
            .keys()
            .find(|name| !offered.is_some_and(|known| known.contains_key(*name)))
        {
            return Err(invalid(format!("{name} is not a parameter of web_search")));
        }
        let text = |name: &str| {
            args.get(name)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|said| !said.is_empty())
                .map(str::to_owned)
        };
        let query = text(QUERY).ok_or_else(|| invalid(format!("{QUERY} is empty or missing")))?;
        let objective = match supplier.objective_field {
            Some(_) => Some(
                text(OBJECTIVE)
                    .ok_or_else(|| invalid(format!("{OBJECTIVE} is empty or missing")))?,
            ),
            None => None,
        };
        let count = match args.get(COUNT) {
            None => None,
            Some(value) => Some(
                value
                    .as_u64()
                    .filter(|count| *count > 0)
                    .ok_or_else(|| invalid(format!("{COUNT} is not a positive integer")))?,
            ),
        };
        Ok(Asked {
            query,
            objective,
            count,
        })
    }

    /// The arguments the remote tool is called with: each asked value
    /// under the remote name `supplier` maps it to.
    ///
    /// # Errors
    /// The refusal [`judge_mapping`] gives, and whatever `Payload` says
    /// about the encoding.
    pub(super) fn mapped(
        &self,
        supplier: &SearchSupplier,
        remote_schema: &Map<String, Value>,
    ) -> Result<Payload, AxError> {
        judge_mapping(supplier, remote_schema)?;
        let mut out = Map::new();
        out.insert(
            supplier.query_field.clone(),
            Value::from(self.query.clone()),
        );
        if let (Some(field), Some(objective)) = (&supplier.objective_field, &self.objective) {
            out.insert(field.clone(), Value::from(objective.clone()));
        }
        if let (Some(field), Some(count)) = (&supplier.count_field, self.count) {
            out.insert(field.clone(), Value::from(count));
        }
        Payload::new(out)
    }
}

/// Whether `supplier`'s mapping fits the remote tool's `inputSchema`:
/// every mapped name is one of its `properties`, and every name it
/// `required` is mapped. The one judgement the call and the settings
/// page's save both ask, so the two cannot disagree about a supplier.
///
/// # Errors
/// `E_TOOL_UNAVAILABLE`, naming the first name that does not fit, with
/// a recovery that points at this supplier's mapping.
pub(crate) fn judge_mapping(
    supplier: &SearchSupplier,
    remote_schema: &Map<String, Value>,
) -> Result<(), AxError> {
    let properties = remote_schema.get("properties").and_then(Value::as_object);
    let mapped: Vec<&String> = std::iter::once(&supplier.query_field)
        .chain(&supplier.objective_field)
        .chain(&supplier.count_field)
        .collect();
    if let Some(name) = mapped
        .iter()
        .find(|name| !properties.is_some_and(|known| known.contains_key(name.as_str())))
    {
        return Err(misfit(
            supplier,
            format!("{} takes no parameter named {name}", supplier.remote),
        ));
    }
    let required = remote_schema
        .get("required")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    if let Some(name) = required
        .iter()
        .filter_map(Value::as_str)
        .find(|name| !mapped.iter().any(|field| field.as_str() == *name))
    {
        return Err(misfit(
            supplier,
            format!(
                "{} requires {name}, which no web_search parameter is mapped to",
                supplier.remote
            ),
        ));
    }
    Ok(())
}

fn misfit(supplier: &SearchSupplier, subject: String) -> AxError {
    AxError::failure(
        AxCode::ToolUnavailable,
        "map web_search onto a search supplier",
        format!("{}: {subject}", supplier.id.as_str()),
    )
    .with_recovery(
        "edit this supplier's mapping in the web search settings: name the remote tool's own \
         parameters for query, objective and count",
    )
}

fn invalid(subject: String) -> AxError {
    AxError::failure(AxCode::InvalidArgs, "search the web", subject).with_recovery(
        "call web_search with a non-empty query, the objective its parameters require, and \
         num_results as a positive integer when you give it",
    )
}
