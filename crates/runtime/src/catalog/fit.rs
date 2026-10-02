// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a call through `call` stands for, and whether its arguments fit
//! the top level of the named tool's input schema
//! (`crates/runtime/spec/Catalog.lean` `resolveCall`, §8-61).
//!
//! Only the top level is checked: the arguments are an object, every
//! `required` key is present, every declared `type` matches, and no key
//! is extra where `additionalProperties` is `false`. Deeper shapes are
//! the tool's own parser's to judge, and it still refuses them.

use std::collections::BTreeMap;

use kernel::{AxCode, AxError, ErrorDraft, Payload, ToolCall, ToolDef, ToolName};
use serde_json::{Map, Value};

use crate::tools::CallTool;

const ACTION: &str = "call a dormant tool";

/// The call `call` stands for, or the one refusal for why it stands for
/// none.
pub(super) fn resolve(
    tools: &BTreeMap<String, ToolDef>,
    call: &ToolCall,
) -> Result<ToolCall, AxError> {
    let given = call.args.as_map();
    let Some(name) = given.get("name").and_then(Value::as_str) else {
        return Err(refused("missing string argument `name`").with_recovery(
            "pass `name`, the tool to run as the dormant index or `describe` spells it, \
             and `args`, an object of its arguments",
        ));
    };
    if name == CallTool::NAME {
        return Err(refused("`call` cannot run itself")
            .with_recovery("name the tool you mean to run; `call` only stands for another tool"));
    }
    let Some(def) = tools.get(name) else {
        return Err(refused(format!("no tool named `{name}` is admitted here"))
            .with_nearby(
                tools
                    .keys()
                    .filter(|known| known.contains(name) || name.contains(known.as_str()))
                    .cloned()
                    .collect(),
            )
            .with_recovery(
                "`describe` a few words of what you need: it answers with the names this \
                 building admits",
            ));
    };
    let args = match given.get("args") {
        Some(Value::Object(args)) => args.clone(),
        None => Map::new(),
        Some(_) => {
            return Err(refused(format!("`args` for `{name}` is not an object"))
                .with_recovery(with_schema(def, "pass `args` as an object")));
        }
    };
    if let Err(misfit) = fits(def.input_schema.as_map(), &args) {
        return Err(refused(format!("`{name}`: {misfit}"))
            .with_recovery(with_schema(def, "call it again with arguments that fit")));
    }
    Ok(ToolCall {
        id: call.id.clone(),
        name: ToolName::parse(name)?,
        args: Payload::new(args)?,
    })
}

fn refused(subject: impl Into<String>) -> ErrorDraft {
    AxError::failure(AxCode::InvalidArgs, ACTION, subject)
}

/// A recovery sentence that hands the model the schema it missed.
fn with_schema(def: &ToolDef, what: &str) -> String {
    let schema = serde_json::to_string(&def.input_schema)
        .unwrap_or_else(|err| format!("(the schema did not print: {err})"));
    format!("{what}; `{}` takes: {schema}", def.name)
}

/// Whether `args` fits the top level of `schema`, or the first reason it
/// does not.
pub(super) fn fits(schema: &Map<String, Value>, args: &Map<String, Value>) -> Result<(), String> {
    let declared = schema.get("properties").and_then(Value::as_object);
    if let Some(missing) = schema
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .find(|key| !args.contains_key(*key))
    {
        return Err(format!("missing required argument `{missing}`"));
    }
    for (key, value) in args {
        match declared.and_then(|properties| properties.get(key)) {
            Some(property) => {
                if let Some(wanted) = property.get("type")
                    && !is_of(value, wanted)
                {
                    return Err(format!("argument `{key}` is not of type {wanted}"));
                }
            }
            None => {
                if schema.get("additionalProperties") == Some(&Value::Bool(false)) {
                    return Err(format!("argument `{key}` is not one this tool takes"));
                }
            }
        }
    }
    Ok(())
}

/// Whether `value` is of the JSON Schema `type` named by `wanted`, a
/// type name or an array of them. A type this check does not know is
/// left to the tool.
fn is_of(value: &Value, wanted: &Value) -> bool {
    match wanted {
        Value::String(name) => is_named(value, name),
        Value::Array(names) => names
            .iter()
            .filter_map(Value::as_str)
            .any(|name| is_named(value, name)),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::Object(_) => true,
    }
}

fn is_named(value: &Value, name: &str) -> bool {
    match name {
        "string" => value.is_string(),
        "integer" => value.is_i64() || value.is_u64(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        "null" => value.is_null(),
        _ => true,
    }
}
