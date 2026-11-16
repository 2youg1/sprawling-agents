// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one drawing every record gets in the detail pane: an indented
//! JSON tree (`crates/sprawling/spec/Main.lean` §8-117). No event kind has a drawing of
//! its own, so a new kind needs no line here.

use serde_json::Value;

const INDENT: &str = "  ";

/// `value` as one line per scalar, nested members indented under their
/// key; object members keep the order the value holds them in.
pub(super) fn json_lines(value: &Value) -> Vec<String> {
    let mut lines = Vec::new();
    push_members(value, "", &mut lines);
    lines
}

/// A Ledger line under its whole chain hash, then drawn as
/// [`json_lines`] draws its JSON; a line that is not JSON is drawn as the
/// one string it is.
pub(super) fn line_lines(line: &str) -> Vec<String> {
    let hash = format!(
        "chain_hash: {}",
        kernel::ledger::chain_hash(line.as_bytes())
    );
    std::iter::once(hash)
        .chain(json_lines(
            &serde_json::from_str(line).unwrap_or_else(|_| Value::String(line.to_owned())),
        ))
        .collect()
}

fn push_members(value: &Value, indent: &str, lines: &mut Vec<String>) {
    match value {
        Value::Object(members) => {
            for (key, member) in members {
                push_named(key, member, indent, lines);
            }
        }
        Value::Array(items) => {
            for (at, item) in items.iter().enumerate() {
                push_named(&format!("[{at}]"), item, indent, lines);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
            lines.push(format!("{indent}{value}"));
        }
    }
}

fn push_named(name: &str, value: &Value, indent: &str, lines: &mut Vec<String>) {
    match value {
        Value::Object(_) | Value::Array(_) => {
            lines.push(format!("{indent}{name}:"));
            push_members(value, &format!("{indent}{INDENT}"), lines);
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
            lines.push(format!("{indent}{name}: {value}"));
        }
    }
}
