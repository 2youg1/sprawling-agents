// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The search tool: one substring, one bounded subtree, and the line
//! number `read` continues from.
//!
//! Before this file the city had thirteen tools and no way to find
//! anything: locating one symbol meant writing Python, which needs the
//! optional CPython-WASI component, or going through a shell a building
//! may have switched off. An address a model cannot search is an
//! address it cannot use.
//!
//! **A substring, never a regular expression.** This follows the ruling
//! already recorded in the `compaction` module header, for its reason
//! rather than by analogy: a pattern engine on a path a model drives
//! puts backtracking between that model and its next turn, and here the
//! pattern would be written by the model itself, so the pathological
//! input is not a rare accident but an ordinary Tuesday.
//!
//! **What may be searched is what may be read.** The prefix goes
//! through `chosen_path::admit`, and every candidate file through
//! `Address::is_reserved`, so a run cannot reach its own governance
//! sideways through a search after `read` closed the front door.

use std::path::{Path, PathBuf};

use kernel::{
    Address, AxCode, AxError, CostTier, Effect, Payload, RenderIntent, Temporal, Tool, ToolCall,
    ToolMeta, ToolName, ToolOutcome,
};
use serde_json::{Map, Value};

use super::chosen_path;

/// How many hits one call may bring back. A search that filled the
/// window would be a search nobody can afford to run twice.
const MATCH_CAP: usize = 64;

/// How large a file may be before the walk passes over it. Reading a
/// large object into memory to look for a substring is a stall, and the
/// material this tool is for is source and prose.
const FILE_BYTE_CAP: u64 = 1 << 20;

/// How many lines of context each side may be asked for.
const CONTEXT_CAP: u64 = 4;

/// What one call is looking for, settled once from the arguments so the
/// walk carries a value rather than four parameters.
struct Predicate {
    needle: String,
    context: usize,
}

/// What the walk has found so far, and what it could not open.
struct Findings {
    hits: Vec<Value>,
    /// Bytes of hit text handed back so far, against
    /// `INTERVAL_CAP_BYTES`. Counted rather than measured at the end,
    /// because the answer has to stop before it is over budget.
    spent: usize,
    unreadable: u64,
    truncated: bool,
}

/// What a search answers with, and what it refuses.
pub struct SearchTool {
    city_root: PathBuf,
    meta: ToolMeta,
}

impl SearchTool {
    /// # Errors
    /// Propagates a malformed parameter schema, which is a build-time
    /// defect rather than a runtime one.
    pub fn new(city_root: &Path) -> Result<SearchTool, AxError> {
        let mut properties = Map::new();
        for (name, description) in [
            (
                "text",
                "the substring to look for, matched literally: this tool has no pattern syntax",
            ),
            (
                "path",
                "a city-relative directory or file to search under; the whole city when omitted",
            ),
            (
                "context",
                "how many lines to show each side of a hit, 0 to 4, default 0",
            ),
        ] {
            let mut spec = Map::new();
            spec.insert(
                "type".to_owned(),
                Value::String(
                    if name == "context" {
                        "integer"
                    } else {
                        "string"
                    }
                    .to_owned(),
                ),
            );
            spec.insert(
                "description".to_owned(),
                Value::String(description.to_owned()),
            );
            properties.insert(name.to_owned(), Value::Object(spec));
        }
        let mut params = Map::new();
        params.insert("type".to_owned(), Value::String("object".to_owned()));
        params.insert("properties".to_owned(), Value::Object(properties));
        params.insert(
            "required".to_owned(),
            Value::Array(vec![Value::String("text".to_owned())]),
        );
        Ok(SearchTool {
            city_root: city_root.to_path_buf(),
            meta: ToolMeta {
                name: ToolName::parse("search")?,
                disclosure: "Find a substring in the files under one path. Literal text, no \
                             regular expressions; the line number it reports is the offset \
                             `read` continues from."
                    .to_owned(),
                params: Payload::new(params)?,
                effect: Effect::Read,
                cost_tier: CostTier::Light,
                timeout: None,
                render: RenderIntent::Generic,
                temporal: Temporal::Timeless,
            },
        })
    }

    /// What this call is looking for, or why it is not a question.
    fn predicate(call: &ToolCall) -> Result<Predicate, AxError> {
        let args = call.args.as_map();
        let needle = args
            .get("text")
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "search",
                    "missing non-empty string argument `text`",
                )
                .with_recovery(
                    "pass the literal text to look for; an empty predicate would match every \
                     line in the city",
                )
            })?;
        let context = match args.get("context") {
            None => 0,
            Some(value) => {
                let asked = value.as_u64().ok_or_else(|| {
                    AxError::failure(
                        AxCode::InvalidArgs,
                        "search",
                        "`context` is not a whole number of lines",
                    )
                    .with_recovery("pass `context` as an integer from 0 to 4, or leave it out")
                })?;
                usize::try_from(asked.min(CONTEXT_CAP)).map_err(|_| {
                    AxError::failure(AxCode::InvalidArgs, "search", "`context` does not fit")
                })?
            }
        };
        Ok(Predicate {
            needle: needle.to_owned(),
            context,
        })
    }

    /// Where the walk starts, as an absolute path and as the city
    /// address that prefixes every hit it reports.
    fn start(&self, call: &ToolCall) -> Result<(PathBuf, String), AxError> {
        match call.args.as_map().get("path").and_then(Value::as_str) {
            None => Ok((self.city_root.clone(), String::new())),
            Some(asked) => {
                let addr = chosen_path::admit(asked, "search")?;
                let mut path = self.city_root.clone();
                for segment in addr.as_str().split('/') {
                    path.push(segment);
                }
                if !path.exists() {
                    return Err(AxError::failure(
                        AxCode::InvalidArgs,
                        "search",
                        format!("{asked} is not a place in this city"),
                    )
                    .with_recovery("name a path that exists, or leave `path` out to search all"));
                }
                Ok((path, addr.as_str().to_owned()))
            }
        }
    }

    /// Walks the subtree in name order, so the same city and the same
    /// question answer the same way twice.
    fn walk(&self, from: (PathBuf, String), looking: &Predicate) -> Findings {
        let mut found = Findings {
            hits: Vec::new(),
            spent: 0,
            unreadable: 0,
            truncated: false,
        };
        let mut pending = vec![from];
        while let Some((path, rel)) = pending.pop() {
            if found.truncated {
                break;
            }
            if path.is_dir() {
                match sorted_entries(&path) {
                    Some(entries) => {
                        // Reversed, because the stack hands back what
                        // was pushed last and the answer is in name
                        // order.
                        for name in entries.into_iter().rev() {
                            if let Some(child) = admissible(&rel, &name) {
                                pending.push((path.join(&name), child));
                            }
                        }
                    }
                    None => found.unreadable = found.unreadable.saturating_add(1),
                }
            } else {
                self.scan_file(&path, &rel, looking, &mut found);
            }
        }
        found
    }

    /// Looks through one file, adding what it finds until the cap.
    fn scan_file(&self, path: &Path, rel: &str, looking: &Predicate, found: &mut Findings) {
        match std::fs::metadata(path) {
            Ok(meta) if meta.len() > FILE_BYTE_CAP => return,
            Ok(_) => {}
            Err(_) => {
                found.unreadable = found.unreadable.saturating_add(1);
                return;
            }
        }
        // A file that is not text has no readable line, so it is passed
        // over rather than counted as a failure: binary content in a
        // city is ordinary, not broken.
        let Ok(body) = std::fs::read_to_string(path) else {
            return;
        };
        let lines: Vec<&str> = body.lines().collect();
        for (at, line) in lines.iter().enumerate() {
            if !line.contains(&looking.needle) {
                continue;
            }
            if found.hits.len() >= MATCH_CAP {
                found.truncated = true;
                return;
            }
            let Ok(number) = u64::try_from(at) else {
                found.unreadable = found.unreadable.saturating_add(1);
                return;
            };
            // Sixty-four hits is a bound on how many answers travel, not
            // on how large they are: nine context lines out of a
            // generated file are nine lines of whatever that file puts
            // on a line. The byte budget is the same ceiling `read`
            // keeps, and for the same reason - one result may not spend
            // the window a caller still has to think in.
            let block = context_block(&lines, at, looking.context);
            let budget =
                usize::try_from(kernel::consts_policy::INTERVAL_CAP_BYTES).unwrap_or(usize::MAX);
            if found.spent.saturating_add(block.len()) > budget && !found.hits.is_empty() {
                found.truncated = true;
                return;
            }
            found.spent = found.spent.saturating_add(block.len());
            let mut hit = Map::new();
            hit.insert("path".to_owned(), Value::String(rel.to_owned()));
            hit.insert("line".to_owned(), Value::Number(number.into()));
            hit.insert("text".to_owned(), Value::String(block));
            found.hits.push(Value::Object(hit));
        }
    }
}

/// The lines around a hit, joined as they were read.
fn context_block(lines: &[&str], at: usize, context: usize) -> String {
    let first = at.saturating_sub(context);
    let last = at
        .saturating_add(context)
        .min(lines.len().saturating_sub(1));
    lines.get(first..=last).unwrap_or_default().join("\n")
}

/// One directory's entries by name, or nothing when it will not open.
fn sorted_entries(path: &Path) -> Option<Vec<String>> {
    let mut names: Vec<String> = std::fs::read_dir(path)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    Some(names)
}

/// The child address the walk may descend to, or nothing.
///
/// Reservedness is asked of `kernel::Address`, the same primitive
/// `read` reaches through `chosen_path`; a name that cannot be an
/// address at all — one holding a backslash, a colon, a control
/// character — is left alone, because this city cannot say where it is.
/// `.git` is skipped for a different reason, stated in its own line:
/// it is an object store, and scanning it yields hits nobody can act on.
fn admissible(rel: &str, name: &str) -> Option<String> {
    if name == ".git" {
        return None;
    }
    let child = if rel.is_empty() {
        name.to_owned()
    } else {
        format!("{rel}/{name}")
    };
    let addr = Address::parse(&child).ok()?;
    if addr.is_reserved() {
        return None;
    }
    Some(child)
}

impl Tool for SearchTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    fn invoke(&mut self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "search",
                format!("call routed to the wrong tool: {}", call.name.as_str()),
            ));
        }
        let looking = SearchTool::predicate(call)?;
        let found = self.walk(self.start(call)?, &looking);
        let count = u64::try_from(found.hits.len()).map_err(|_| {
            AxError::failure(AxCode::StorageFatal, "search", "match count overflow")
        })?;
        let mut out = Map::new();
        out.insert("count".to_owned(), Value::Number(count.into()));
        out.insert("truncated".to_owned(), Value::Bool(found.truncated));
        // What the walk could not open, rather than silence about it: a
        // search that reports nothing is a different answer from a
        // search that could not look.
        out.insert(
            "unreadable".to_owned(),
            Value::Number(found.unreadable.into()),
        );
        out.insert("matches".to_owned(), Value::Array(found.hits));
        Ok(ToolOutcome {
            result: Payload::new(out)?,
            attachments: Vec::new(),
        })
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
