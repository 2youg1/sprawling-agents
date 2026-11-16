// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use super::Shape;

pub(super) fn read(shape: Shape, text: &str) -> Option<BTreeMap<String, String>> {
    match shape {
        Shape::Json => leaves(&serde_json::from_str::<serde_json::Value>(text).ok()?),
        Shape::Toml => leaves(&toml::from_str::<serde_json::Value>(text).ok()?),
        Shape::Markdown => markdown_leaves(text),
    }
}

fn leaves(value: &serde_json::Value) -> Option<BTreeMap<String, String>> {
    let mut found = BTreeMap::new();
    walk(value, &mut String::new(), &mut found);
    Some(found)
}

fn walk(value: &serde_json::Value, at: &mut String, into: &mut BTreeMap<String, String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (name, held) in map {
                let was = at.len();
                if !at.is_empty() {
                    at.push('/');
                }
                at.push_str(name);
                walk(held, at, into);
                at.truncate(was);
            }
        }
        serde_json::Value::Array(items) => {
            for (index, held) in items.iter().enumerate() {
                let was = at.len();
                if !at.is_empty() {
                    at.push('/');
                }
                at.push_str(&index.to_string());
                walk(held, at, into);
                at.truncate(was);
            }
        }
        other @ (serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_)) => {
            into.insert(at.clone(), scalar(other));
        }
    }
}

fn scalar(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        other @ (serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::Array(_)
        | serde_json::Value::Object(_)) => other.to_string(),
    }
}

fn markdown_leaves(text: &str) -> Option<BTreeMap<String, String>> {
    let mut found = BTreeMap::new();
    let mut stack: Vec<String> = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let indent = line.len().checked_sub(line.trim_start().len())?;

        if indent % 2 != 0 {
            return None;
        }
        let depth = indent.checked_div(2)?;
        let item = line.trim_start().strip_prefix("- ")?;
        let (name, value) = match item.split_once(": ") {
            Some((name, value)) => (name.trim(), Some(value.trim())),
            None => (item.trim(), None),
        };
        if name.is_empty() {
            return None;
        }
        stack.truncate(depth);
        if stack.len() != depth {
            return None;
        }
        stack.push(name.to_owned());
        if let Some(value) = value {
            found.insert(stack.join("/"), value.to_owned());
        }
    }
    Some(found)
}

pub(super) fn stops_early(shape: Shape, text: &str) -> bool {
    match shape {
        Shape::Json => {
            let mut depth: i64 = 0;
            let mut inside = false;
            let mut escaped = false;
            for glyph in text.chars() {
                match glyph {
                    _ if escaped => escaped = false,
                    '\\' if inside => escaped = true,
                    '"' => inside = !inside,
                    '{' | '[' if !inside => depth = depth.saturating_add(1),
                    '}' | ']' if !inside => depth = depth.saturating_sub(1),
                    _ => {}
                }
            }
            depth > 0 || inside
        }

        Shape::Toml => {
            let last = text.lines().last().unwrap_or_default().trim();
            last.starts_with('[') && !last.ends_with(']')
                || last.matches('"').count() % 2 == 1
                || last.ends_with('=')
        }

        Shape::Markdown => {
            let last = text.lines().last().unwrap_or_default();
            last.trim_start().starts_with("- ") && !last.contains(": ")
        }
    }
}
