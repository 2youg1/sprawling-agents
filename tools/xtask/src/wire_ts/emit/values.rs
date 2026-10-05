// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One schema in, one Effect 4 expression out
//! (`tools/xtask/Spec.lean` section 8-52).
//!
//! The half of the emitter that reads a schema rather than a document:
//! which keywords the subset allows, and what each one becomes. Naming,
//! ordering and the file's preamble are `emit`'s; everything here is
//! about the value a single schema describes, and the two meet at
//! `expression`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use serde_json::{Map, Value};

use super::{REF_PREFIX, Refused, is_identifier, refuse};

/// The definitions a reference reaches lazily from the one being
/// written: the recursive ones, when it is recursive itself, and none
/// otherwise, because a definition outside a cycle comes after
/// everything it refers to.
pub(super) struct Values<'a> {
    pub(super) suspended: &'a BTreeSet<String>,
}

/// Every keyword the emitter reads; one it does not know is a refusal.
const KNOWN: &str = concat!(
    "$ref additionalProperties anyOf const default description enum format items ",
    "maxItems maximum minItems minimum oneOf pattern prefixItems properties required type",
);

impl Values<'_> {
    /// The Effect expression for one schema. `at` is where it sits, for the
    /// refusal; `indent` is how deep its own lines start.
    pub(super) fn expression(
        &self,
        schema: &Value,
        at: &str,
        indent: usize,
    ) -> Result<String, Refused> {
        let map = match schema {
            Value::Bool(true) => return Ok("Schema.Unknown".to_owned()),
            Value::Bool(false) => return Ok("Schema.Never".to_owned()),
            Value::Object(map) => map,
            _ => return Err(refuse(at, "a schema is an object or a boolean")),
        };
        if let Some(unknown) = map.keys().find(|k| {
            !KNOWN
                .split_ascii_whitespace()
                .any(|known| known == k.as_str())
        }) {
            return Err(refuse(
                at,
                format!("keyword `{unknown}` is outside the subset serde produces for this wire"),
            ));
        }
        if let Some(target) = map.get("$ref") {
            return self.reference(target, at);
        }
        if let Some(members) = map.get("oneOf").or_else(|| map.get("anyOf")) {
            return self.union(members, at, indent);
        }
        if let Some(values) = map.get("enum") {
            return literals(values, at);
        }
        if let Some(value) = map.get("const") {
            return literals(&Value::Array(vec![value.clone()]), at);
        }
        match map.get("type") {
            Some(kind) => self.typed(map, kind, at, indent),
            None => Err(refuse(
                at,
                "a schema with no `type`, `$ref`, `enum`, `const` or union",
            )),
        }
    }

    fn reference(&self, target: &Value, at: &str) -> Result<String, Refused> {
        let name = target
            .as_str()
            .and_then(|t| t.strip_prefix(REF_PREFIX))
            .filter(|name| is_identifier(name))
            .ok_or_else(|| refuse(at, format!("`$ref` {target} does not point into `$defs`")))?;
        Ok(if self.suspended.contains(name) {
            format!("Schema.suspend((): Schema.Codec<{name}, {name}Encoded> => {name})")
        } else {
            name.to_owned()
        })
    }

    fn union(&self, members: &Value, at: &str, indent: usize) -> Result<String, Refused> {
        let members = members
            .as_array()
            .ok_or_else(|| refuse(at, "`oneOf`/`anyOf` is not a list"))?;
        // `Option<T>` on a `$ref` arrives as `anyOf: [T, null]`; it is one
        // nullable value, not a union a reader has to take apart.
        if let [one, two] = members.as_slice() {
            let null = serde_json::json!({ "type": "null" });
            if two == &null {
                return Ok(format!(
                    "Schema.NullOr({})",
                    self.expression(one, at, indent)?
                ));
            }
            if one == &null {
                return Ok(format!(
                    "Schema.NullOr({})",
                    self.expression(two, at, indent)?
                ));
            }
        }
        let mut out = String::from("Schema.Union([\n");
        for (index, member) in members.iter().enumerate() {
            let member = self.expression(
                member,
                &format!("{at}/oneOf[{index}]"),
                indent.saturating_add(1),
            )?;
            let _ = writeln!(out, "{}{member},", pad(indent.saturating_add(1)));
        }
        let _ = write!(out, "{}])", pad(indent));
        Ok(out)
    }

    fn typed(
        &self,
        map: &Map<String, Value>,
        kind: &Value,
        at: &str,
        indent: usize,
    ) -> Result<String, Refused> {
        let (kind, nullable) = match kind {
            Value::String(kind) => (kind.as_str(), false),
            Value::Array(kinds) => {
                let kinds: Vec<&str> = kinds.iter().filter_map(Value::as_str).collect();
                let rest: Vec<&str> = kinds.iter().copied().filter(|k| *k != "null").collect();
                match rest.as_slice() {
                    [one] => (*one, kinds.len() > rest.len()),
                    _ => {
                        return Err(refuse(
                            at,
                            format!("type list {kinds:?} names more than one type"),
                        ));
                    }
                }
            }
            _ => return Err(refuse(at, "`type` is neither a string nor a list")),
        };
        let inner = match kind {
            // A grammar the Rust type owns reaches the client only if it is
            // carried here; a client that spelled the same grammar again
            // would be a second home for it. The `u` flag is not optional:
            // without it `\p{Cc}` is the letter `p` in most engines.
            "string" => match map.get("pattern") {
                None => "Schema.String".to_owned(),
                Some(Value::String(pattern)) => format!(
                    "Schema.String.check(Schema.isPattern(new RegExp({}, \"u\")))",
                    serde_json::to_string(pattern).map_err(|e| refuse(at, e.to_string()))?
                ),
                Some(other) => {
                    return Err(refuse(at, format!("`pattern` {other} is not a string")));
                }
            },
            "integer" => numeric_bounds("Schema.Int", map, at)?,
            "number" => numeric_bounds("Schema.Number", map, at)?,
            "boolean" => "Schema.Boolean".to_owned(),
            "null" => "Schema.Null".to_owned(),
            "array" => self.array(map, at, indent)?,
            "object" => self.object(map, at, indent)?,
            other => return Err(refuse(at, format!("type `{other}` is outside the subset"))),
        };
        Ok(if nullable {
            format!("Schema.NullOr({inner})")
        } else {
            inner
        })
    }

    fn array(&self, map: &Map<String, Value>, at: &str, indent: usize) -> Result<String, Refused> {
        if let Some(elements) = map.get("prefixItems").and_then(Value::as_array) {
            let mut spelled = Vec::new();
            for (index, element) in elements.iter().enumerate() {
                spelled.push(self.expression(
                    element,
                    &format!("{at}/prefixItems[{index}]"),
                    indent,
                )?);
            }
            return Ok(format!("Schema.Tuple([{}])", spelled.join(", ")));
        }
        let item = match map.get("items") {
            Some(items) => self.expression(items, &format!("{at}/items"), indent)?,
            None => "Schema.Unknown".to_owned(),
        };
        Ok(format!("Schema.Array({item})"))
    }

    fn object(&self, map: &Map<String, Value>, at: &str, indent: usize) -> Result<String, Refused> {
        if let Some(properties) = map.get("properties").and_then(Value::as_object) {
            return self.fields(properties, map.get("required"), at, indent);
        }
        match map.get("additionalProperties") {
            Some(Value::Bool(false)) => Ok("Schema.Struct({})".to_owned()),
            Some(values) => {
                let value =
                    self.expression(values, &format!("{at}/additionalProperties"), indent)?;
                Ok(format!("Schema.Record(Schema.String, {value})"))
            }
            None => Ok("Schema.Record(Schema.String, Schema.Unknown)".to_owned()),
        }
    }

    fn fields(
        &self,
        properties: &Map<String, Value>,
        required: Option<&Value>,
        at: &str,
        indent: usize,
    ) -> Result<String, Refused> {
        let required: BTreeSet<&str> = required
            .and_then(Value::as_array)
            .map(|list| list.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        let sorted: BTreeMap<&str, &Value> =
            properties.iter().map(|(k, v)| (k.as_str(), v)).collect();
        let mut out = String::from("Schema.Struct({\n");
        for (key, schema) in sorted {
            let value = self.expression(
                schema,
                &format!("{at}/properties/{key}"),
                indent.saturating_add(1),
            )?;
            let value = if required.contains(key) {
                value
            } else {
                format!("Schema.optional({value})")
            };
            let key = if is_identifier(key) {
                key.to_owned()
            } else {
                serde_json::to_string(key).map_err(|e| refuse(at, e.to_string()))?
            };
            let _ = writeln!(out, "{}{key}: {value},", pad(indent.saturating_add(1)));
        }
        let _ = write!(out, "{}}})", pad(indent));
        Ok(out)
    }
}

fn numeric_bounds(base: &str, map: &Map<String, Value>, at: &str) -> Result<String, Refused> {
    let mut expression = base.to_owned();
    for (keyword, filter) in [
        ("minimum", "isGreaterThanOrEqualTo"),
        ("maximum", "isLessThanOrEqualTo"),
    ] {
        if let Some(bound) = map.get(keyword) {
            if !bound.is_number() {
                return Err(refuse(at, format!("`{keyword}` {bound} is not a number")));
            }
            expression.push_str(&format!(".check(Schema.{filter}({bound}))"));
        }
    }
    Ok(expression)
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
    // Effect 4 spells one value and a list of values apart.
    Ok(match spelled.as_slice() {
        [one] => format!("Schema.Literal({one})"),
        _ => format!("Schema.Literals([{}])", spelled.join(", ")),
    })
}

fn pad(indent: usize) -> String {
    "  ".repeat(indent)
}
