// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::report::XtaskError;

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Manifest {
    pub(super) runtime: BTreeMap<String, String>,
    pub(super) development: BTreeMap<String, String>,
}

pub(super) fn read_jsonc(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_string = false;
    while let Some(here) = chars.next() {
        if in_string {
            out.push(here);
            match here {
                '\\' => {
                    if let Some(escaped) = chars.next() {
                        out.push(escaped);
                    }
                }
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match here {
            '"' => {
                in_string = true;
                out.push(here);
            }
            '/' if chars.peek() == Some(&'/') => {
                for skipped in chars.by_ref() {
                    if skipped == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut previous = '\0';
                for skipped in chars.by_ref() {
                    if previous == '*' && skipped == '/' {
                        break;
                    }
                    previous = skipped;
                }
            }
            ',' => {
                let (closing, skipped) = closes_next(&mut chars);
                if !closing {
                    out.push(here);
                }
                out.push_str(&skipped);
            }
            _ => out.push(here),
        }
    }
    out
}

fn closes_next(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> (bool, String) {
    let mut skipped = String::new();
    while let Some(next) = chars.peek() {
        if next.is_whitespace() {
            skipped.push(*next);
            chars.next();
            continue;
        }
        return (*next == '}' || *next == ']', skipped);
    }
    (false, skipped)
}

pub(super) fn document(file: &str, text: &str) -> Result<Value, XtaskError> {
    serde_json::from_str(&read_jsonc(text)).map_err(|err| XtaskError::Doc {
        file: file.to_owned(),
        msg: format!("cannot be read as JSON: {err}"),
    })
}

pub(super) fn manifest_of(document: &Value) -> Manifest {
    Manifest {
        runtime: table(document.get("dependencies")),
        development: table(document.get("devDependencies")),
    }
}

pub(super) fn lock_of(document: &Value) -> Result<Manifest, XtaskError> {
    let root = document
        .get("workspaces")
        .and_then(|held| held.get(""))
        .ok_or_else(|| XtaskError::Doc {
            file: "RefRain/bun.lock".to_owned(),
            msg: "no `workspaces` entry for the root package".to_owned(),
        })?;
    Ok(manifest_of(root))
}

fn table(value: Option<&Value>) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let Some(Value::Object(entries)) = value else {
        return out;
    };
    for (name, held) in entries {
        if let Value::String(range) = held {
            out.insert(name.clone(), range.clone());
        }
    }
    out
}

pub(super) fn permitted(text: &str) -> Result<BTreeSet<String>, XtaskError> {
    let parsed: toml::Value = toml::from_str(text).map_err(|err| XtaskError::Doc {
        file: "deny.toml".to_owned(),
        msg: format!("cannot be read as TOML: {err}"),
    })?;
    let allowed = parsed
        .get("licenses")
        .and_then(|held| held.get("allow"))
        .and_then(toml::Value::as_array)
        .ok_or_else(|| XtaskError::Doc {
            file: "deny.toml".to_owned(),
            msg: "no `[licenses] allow` list".to_owned(),
        })?;
    let out: BTreeSet<String> = allowed
        .iter()
        .filter_map(toml::Value::as_str)
        .map(str::to_owned)
        .collect();
    if out.is_empty() {
        return Err(XtaskError::Doc {
            file: "deny.toml".to_owned(),
            msg: "the `[licenses] allow` list is empty".to_owned(),
        });
    }
    Ok(out)
}

pub(super) fn allowed(expression: &str, permitted: &BTreeSet<String>) -> bool {
    let spaced = expression.replace('(', " ( ").replace(')', " ) ");
    let mut spdx = Spdx {
        tokens: spaced.split_whitespace().peekable(),
        permitted,
    };
    let verdict = spdx.any_of();
    verdict == Some(true) && spdx.tokens.next().is_none()
}

struct Spdx<'a> {
    tokens: std::iter::Peekable<std::str::SplitWhitespace<'a>>,
    permitted: &'a BTreeSet<String>,
}

impl Spdx<'_> {
    fn any_of(&mut self) -> Option<bool> {
        let mut verdict = self.all_of()?;
        while self.tokens.next_if_eq(&"OR").is_some() {
            verdict = self.all_of()? || verdict;
        }
        Some(verdict)
    }

    fn all_of(&mut self) -> Option<bool> {
        let mut verdict = self.licence()?;
        while self.tokens.next_if_eq(&"AND").is_some() {
            verdict = self.licence()? && verdict;
        }
        Some(verdict)
    }

    fn licence(&mut self) -> Option<bool> {
        match self.tokens.next()? {
            "(" => {
                let verdict = self.any_of()?;
                self.tokens.next_if_eq(&")")?;
                Some(verdict)
            }
            ")" | "AND" | "OR" | "WITH" => None,
            identifier => {
                if self.tokens.next_if_eq(&"WITH").is_none() {
                    return Some(self.permitted.contains(identifier));
                }
                let exception = self
                    .tokens
                    .next()
                    .filter(|word| !matches!(*word, "(" | ")" | "AND" | "OR" | "WITH"))?;
                Some(
                    self.permitted
                        .contains(&format!("{identifier} WITH {exception}")),
                )
            }
        }
    }
}
