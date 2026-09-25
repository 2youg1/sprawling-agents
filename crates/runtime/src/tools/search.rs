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
//! through `chosen_path::admit`, every candidate file through
//! `Address::is_reserved`, and every building the walk enters from the
//! city root through the read bound, so a run cannot reach its own
//! governance or a confidential building sideways through a search after
//! `read` closed the front door.

use std::path::{Path, PathBuf};

use kernel::{
    Address, AxCode, AxError, CostTier, Effect, Payload, ReadVerdict, RenderIntent, Temporal, Tool,
    ToolCall, ToolMeta, ToolName, ToolOutcome,
};
use serde_json::{Map, Value};

use super::chosen_path::{self, ReadBound};
use crate::elision::{self, Elided};

/// How many hits one call may bring back. A search that filled the
/// window would be a search nobody can afford to run twice.
const MATCH_CAP: usize = 64;

/// How large a file may be before the walk passes over it. Reading a
/// large object into memory to look for a substring is a stall, and the
/// material this tool is for is source and prose. A file past it is
/// reported, because `read` can still take it an interval at a time.
const FILE_BYTE_CAP: u64 = 1 << 20;

/// How many of the places the walk could not look through are named:
/// the count covers all of them, and a model acts on a few names.
const UNREAD_SHOWN: usize = 16;

/// How many lines of context each side may be asked for.
const CONTEXT_CAP: u8 = 4;

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
    /// The first [`UNREAD_SHOWN`] of those, each with its reason.
    unread: Vec<Value>,
    truncated: bool,
}

impl Findings {
    /// Counts a place the walk could not look through, and names it
    /// while there is room.
    fn could_not_look(&mut self, rel: &str, why: String) {
        self.unreadable = self.unreadable.saturating_add(1);
        if self.unread.len() < UNREAD_SHOWN {
            self.unread
                .push(serde_json::json!({ "path": rel, "why": why }));
        }
    }
}

/// What a search answers with, and what it refuses.
pub struct SearchTool {
    city_root: PathBuf,
    /// What this run's building may read: the prefix is admitted by it,
    /// and a walk from the city root enters only the buildings it opens.
    bound: ReadBound,
    meta: ToolMeta,
}

impl SearchTool {
    /// # Errors
    /// Propagates a malformed parameter schema, which is a build-time
    /// defect rather than a runtime one.
    pub fn new(city_root: &Path, bound: ReadBound) -> Result<SearchTool, AxError> {
        Ok(SearchTool {
            city_root: city_root.to_path_buf(),
            bound,
            meta: ToolMeta {
                name: ToolName::parse("search")?,
                disclosure: "Find a substring in the files under one path. Literal text, no \
                             regular expressions; the line number it reports is the offset \
                             `read` continues from. A `grep` through `exec` returns the same \
                             text without the version and without the offset."
                    .to_owned(),
                params: Payload::of(&serde_json::json!({
                    "type": "object",
                    "properties": {
                        "text": { "type": "string", "description": "the substring to look for, \
                            matched literally: this tool has no pattern syntax" },
                        "path": { "type": "string", "description": "a city-relative directory \
                            or file to search under; everything you may read when omitted" },
                        "context": { "type": "integer", "description": "how many lines to \
                            show each side of a hit, 0 to 4, default 0" },
                    },
                    "required": ["text"],
                }))?,
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
                // Above the cap is served at the cap, and so is a number
                // no `u8` holds.
                usize::from(u8::try_from(asked).map_or(CONTEXT_CAP, |fits| fits.min(CONTEXT_CAP)))
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
                let addr = chosen_path::admit(asked, "search", &*self.bound)?;
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
            unread: Vec::new(),
            truncated: false,
        };
        let mut pending = vec![from];
        while let Some((path, rel)) = pending.pop() {
            if found.truncated {
                break;
            }
            if path.is_dir() {
                match sorted_entries(&path) {
                    Ok(entries) => {
                        // Reversed, because the stack hands back what
                        // was pushed last and the answer is in name
                        // order.
                        for name in entries.into_iter().rev() {
                            if let Some(child) = admissible(&rel, &name, &*self.bound) {
                                pending.push((path.join(&name), child));
                            }
                        }
                    }
                    Err(err) => found.could_not_look(&rel, format!("would not open: {err}")),
                }
            } else {
                self.scan_file(&path, &rel, looking, &mut found);
            }
        }
        found
    }

    /// Looks through one file, adding what it finds until the cap.
    fn scan_file(&self, path: &Path, rel: &str, looking: &Predicate, found: &mut Findings) {
        let opened = std::fs::metadata(path).and_then(|meta| {
            if meta.len() > FILE_BYTE_CAP {
                Ok(None)
            } else {
                std::fs::read(path).map(Some)
            }
        });
        let bytes = match opened {
            Ok(Some(bytes)) => bytes,
            Ok(None) => {
                let why = format!(
                    "larger than the {FILE_BYTE_CAP} bytes one search reads; `read` it an \
                     interval at a time"
                );
                return found.could_not_look(rel, why);
            }
            Err(err) => return found.could_not_look(rel, format!("would not open: {err}")),
        };
        // A file that is not text has no readable line, so it is passed
        // over rather than counted as a failure: binary content in a
        // city is ordinary, not broken.
        let Ok(body) = String::from_utf8(bytes) else {
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
            // Sixty-four hits is a bound on how many answers travel, not
            // on how large they are: nine context lines out of a
            // generated file are nine lines of whatever that file puts
            // on a line. The byte budget is the same ceiling `read`
            // keeps, and for the same reason - one result may not spend
            // the window a caller still has to think in.
            let block = context_block(&lines, at, looking.context);
            let budget = kernel::consts_policy::INTERVAL_CAP_BYTES;
            let fits = found.spent.saturating_add(block.len()) <= budget;
            if !fits && !found.hits.is_empty() {
                found.truncated = true;
                return;
            }
            // The first hit is held to the budget as well, cut at its
            // tail and marked: its line number is where `read` takes up
            // the rest, so nothing is lost by cutting it.
            let text = if fits {
                block
            } else {
                found.truncated = true;
                let keep = budget.saturating_sub(elision::marker_room(block.len()));
                elision::splice(&block, keep, block.len(), Elided::Tail).text
            };
            found.spent = found.spent.saturating_add(text.len());
            found
                .hits
                .push(serde_json::json!({ "path": rel, "line": at, "text": text }));
            if found.truncated {
                return;
            }
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

/// One directory's entries by name, or why it will not open.
fn sorted_entries(path: &Path) -> std::io::Result<Vec<String>> {
    let mut names: Vec<String> = std::fs::read_dir(path)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    Ok(names)
}

/// The child address the walk may descend to, or nothing.
///
/// Reservedness is asked of `kernel::Address`, the same primitive
/// `read` reaches through `chosen_path`; a name that cannot be an
/// address at all — one holding a backslash, a colon, a control
/// character — is left alone, because this city cannot say where it is.
/// Git's own metadata is inside that predicate too (kernel-SPEC 8-73),
/// and scanning an object store yields hits nobody can act on.
///
/// The read bound is asked only at the city root: it answers for a
/// building, a building is a top-level address, and all below one
/// shares its answer, so no file costs another read of the rules.
fn admissible(rel: &str, name: &str, bound: &dyn Fn(&Address) -> ReadVerdict) -> Option<String> {
    let child = if rel.is_empty() {
        name.to_owned()
    } else {
        format!("{rel}/{name}")
    };
    let addr = Address::parse(&child).ok()?;
    if addr.is_reserved() {
        return None;
    }
    if rel.is_empty() && !matches!(bound(&addr), ReadVerdict::Open) {
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
            )
            .with_recovery(format!(
                "call `{}`, the name this tool answers to",
                self.meta.name.as_str()
            )));
        }
        let looking = SearchTool::predicate(call)?;
        let found = self.walk(self.start(call)?, &looking);
        let mut out = Map::new();
        out.insert("count".to_owned(), Value::Number(found.hits.len().into()));
        out.insert("truncated".to_owned(), Value::Bool(found.truncated));
        // What the walk could not open, rather than silence about it: a
        // search that reports nothing is a different answer from a
        // search that could not look.
        out.insert(
            "unreadable".to_owned(),
            Value::Number(found.unreadable.into()),
        );
        out.insert("unread".to_owned(), Value::Array(found.unread));
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
