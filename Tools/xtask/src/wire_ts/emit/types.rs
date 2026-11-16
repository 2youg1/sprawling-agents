// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One schema in, the TypeScript type it denotes out.
//!
//! Only a recursive definition needs this: TypeScript cannot infer the
//! type of a value whose initialiser refers to itself, so Effect asks
//! for `Schema.Codec<X, XEncoded>` written out. Every keyword here
//! mirrors the expression `values` writes for the same schema, because
//! the annotation and the value are checked against each other and a
//! difference is a type error in the client.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use super::{REF_PREFIX, Refused, is_identifier, refuse};

/// Which of a schema's two types: the value after decoding, or the JSON
/// before it. They differ only where a newtype carries its brand.
#[derive(Clone, Copy)]
pub(super) enum Side {
    Type,
    Encoded,
}

/// The names that are written out as types of their own, and the side
/// being read.
pub(super) struct Types<'a> {
    pub(super) recursive: &'a BTreeSet<String>,
    pub(super) side: Side,
}

impl Types<'_> {
    /// The type `schema` denotes on this side. `at` is where it sits,
    /// for the refusal.
    pub(super) fn denoted(&self, schema: &Value, at: &str) -> Result<String, Refused> {
        let map = match schema {
            Value::Bool(true) => return Ok("unknown".to_owned()),
            Value::Bool(false) => return Ok("never".to_owned()),
            Value::Object(map) => map,
            _ => return Err(refuse(at, "a schema is an object or a boolean")),
        };
        if let Some(target) = map.get("$ref") {
            return self.reference(target, at);
        }
        if let Some(members) = map.get("oneOf").or_else(|| map.get("anyOf")) {
            return self.union(members, at);
        }
        if let Some(values) = map.get("enum") {
            return literals(values, at);
        }
        if let Some(value) = map.get("const") {
            return literals(&Value::Array(vec![value.clone()]), at);
        }
        match map.get("type") {
            Some(Value::String(kind)) => self.typed(map, kind, at),
            Some(Value::Array(kinds)) => {
                let rest: Vec<&str> = kinds
                    .iter()
                    .filter_map(Value::as_str)
                    .filter(|k| *k != "null")
                    .collect();
                match rest.as_slice() {
                    [one] if rest.len() < kinds.len() => {
                        Ok(format!("{} | null", self.typed(map, one, at)?))
                    }
                    [one] => self.typed(map, one, at),
                    _ => Err(refuse(at, "a type list names more than one type")),
                }
            }
            _ => Err(refuse(at, "a schema with no type this emitter reads")),
        }
    }

    fn reference(&self, target: &Value, at: &str) -> Result<String, Refused> {
        let name = target
            .as_str()
            .and_then(|t| t.strip_prefix(REF_PREFIX))
            .filter(|name| is_identifier(name))
            .ok_or_else(|| refuse(at, format!("`$ref` {target} does not point into `$defs`")))?;
        Ok(match (self.recursive.contains(name), self.side) {
            (true, Side::Type) => name.to_owned(),
            (true, Side::Encoded) => format!("{name}Encoded"),
            (false, Side::Type) => format!("typeof {name}.Type"),
            (false, Side::Encoded) => format!("typeof {name}.Encoded"),
        })
    }

    fn union(&self, members: &Value, at: &str) -> Result<String, Refused> {
        let members = members
            .as_array()
            .ok_or_else(|| refuse(at, "`oneOf`/`anyOf` is not a list"))?;
        let mut spelled = Vec::new();
        for (index, member) in members.iter().enumerate() {
            spelled.push(self.denoted(member, &format!("{at}/oneOf[{index}]"))?);
        }
        Ok(spelled.join(" | "))
    }

    fn typed(&self, map: &Map<String, Value>, kind: &str, at: &str) -> Result<String, Refused> {
        match kind {
            "string" => Ok("string".to_owned()),
            "integer" | "number" => Ok("number".to_owned()),
            "boolean" => Ok("boolean".to_owned()),
            "null" => Ok("null".to_owned()),
            "array" => self.array(map, at),
            "object" => self.object(map, at),
            other => Err(refuse(at, format!("type `{other}` is outside the subset"))),
        }
    }

    fn array(&self, map: &Map<String, Value>, at: &str) -> Result<String, Refused> {
        if let Some(elements) = map.get("prefixItems").and_then(Value::as_array) {
            let mut spelled = Vec::new();
            for (index, element) in elements.iter().enumerate() {
                spelled.push(self.denoted(element, &format!("{at}/prefixItems[{index}]"))?);
            }
            return Ok(format!("readonly [{}]", spelled.join(", ")));
        }
        let item = match map.get("items") {
            Some(items) => self.denoted(items, &format!("{at}/items"))?,
            None => "unknown".to_owned(),
        };
        Ok(if item.contains(' ') {
            format!("readonly ({item})[]")
        } else {
            format!("readonly {item}[]")
        })
    }

    fn object(&self, map: &Map<String, Value>, at: &str) -> Result<String, Refused> {
        let Some(properties) = map.get("properties").and_then(Value::as_object) else {
            return match map.get("additionalProperties") {
                Some(Value::Bool(false)) => Ok("{}".to_owned()),
                Some(values) => Ok(format!(
                    "Readonly<Record<string, {}>>",
                    self.denoted(values, &format!("{at}/additionalProperties"))?
                )),
                None => Ok("Readonly<Record<string, unknown>>".to_owned()),
            };
        };
        let required: BTreeSet<&str> = map
            .get("required")
            .and_then(Value::as_array)
            .map(|list| list.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        let sorted: BTreeMap<&str, &Value> =
            properties.iter().map(|(k, v)| (k.as_str(), v)).collect();
        let mut fields = Vec::new();
        for (key, schema) in sorted {
            let value = self.denoted(schema, &format!("{at}/properties/{key}"))?;
            let spelled = if is_identifier(key) {
                key.to_owned()
            } else {
                serde_json::to_string(key).map_err(|e| refuse(at, e.to_string()))?
            };
            fields.push(if required.contains(key) {
                format!("readonly {spelled}: {value}")
            } else {
                format!("readonly {spelled}?: {value} | undefined")
            });
        }
        Ok(format!("{{ {} }}", fields.join("; ")))
    }
}

fn literals(values: &Value, at: &str) -> Result<String, Refused> {
    let values = values
        .as_array()
        .ok_or_else(|| refuse(at, "`enum` is not a list"))?;
    let mut spelled = Vec::new();
    for value in values {
        let text = value
            .as_str()
            .ok_or_else(|| refuse(at, format!("literal {value} is not a string")))?;
        spelled.push(serde_json::to_string(text).map_err(|e| refuse(at, e.to_string()))?);
    }
    Ok(spelled.join(" | "))
}

/// The fields of a type that is one object and nothing else, which the
/// client's lint wants written as an `interface`; `None` for a union,
/// an array or any other type.
pub(super) fn object_body(denoted: &str) -> Option<&str> {
    let inner = denoted.strip_prefix("{ ")?.strip_suffix(" }")?;
    let mut depth: usize = 0;
    for c in inner.chars() {
        match c {
            '{' => depth = depth.checked_add(1)?,
            '}' => depth = depth.checked_sub(1)?,
            _ => {}
        }
    }
    (depth == 0).then_some(inner)
}
