// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The search tool: Claude Code's `Grep` over one bounded subtree, and
//! the line number `read` continues from. What a call asks, and which of
//! its arguments are answered, is `predicate`'s to read.
//!
//! **What may be searched is what may be read.** The prefix goes
//! through `chosen_path::admit` and `chosen_path::land`, every candidate
//! file through `Address::is_reserved`, every link through `land`, and
//! every building the walk enters from the city root through the read
//! bound, so a run cannot reach its own governance or a confidential
//! building sideways through a search after `read` closed the front door.

use std::path::{Path, PathBuf};

use kernel::{
    AxCode, AxError, CostTier, Effect, Payload, RenderIntent, Temporal, Tool, ToolCall, ToolMeta,
    ToolName, ToolOutcome,
};
use serde_json::{Map, Value};

mod descent;
mod hit;
mod predicate;

use super::chosen_path::{self, ReadBound, Walked};
use crate::elision::{self, Elided};
use descent::{Entry, Step, admissible};
use hit::{Limit, context_block};
use predicate::{Lines, Mode, Predicate};

/// How many entries one call may bring back: hits in `content`, files in
/// `files_with_matches`, in `count` and for a glob alone. A search that filled the window would be a
/// search nobody can afford to run twice.
const MATCH_CAP: usize = 64;

/// How large a file may be before the walk passes over it. Reading a
/// large object into memory to look for a match is a stall, and the
/// material this tool is for is source and prose. A file past it is
/// reported, because `read` can still take it an interval at a time.
const FILE_BYTE_CAP: u64 = 1 << 20;

/// How many of the places the walk could not look through are named:
/// the count covers all of them, and a model acts on a few names.
const UNREAD_SHOWN: usize = 16;

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
    /// The limit the walk stopped at, or `None` when it looked everywhere.
    stopped: Option<Limit>,
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

    /// Lists one file in a `files` or `count` answer, or stops the walk
    /// when [`MATCH_CAP`] files are already listed.
    fn list(&mut self, entry: Value) {
        if self.hits.len() >= MATCH_CAP {
            self.stopped = Some(Limit::Files);
            return;
        }
        self.hits.push(entry);
    }
}

/// What a search answers with, and what it refuses.
pub struct SearchTool {
    /// The root the walk starts from, anchored once when the tool is
    /// built (`crates/runtime/spec/Tools/ChosenPath.lean` D112).
    city: chosen_path::CityRoot,
    /// What this run's building may read: the prefix is admitted by it,
    /// and a walk from the city root enters only the buildings it opens.
    bound: ReadBound,
    meta: ToolMeta,
}

impl SearchTool {
    /// The name this tool answers to, read by its registration and by
    /// `mode::core_tools`.
    pub const NAME: &'static str = "search";

    /// # Errors
    /// Propagates a malformed parameter schema, which is a build-time
    /// defect rather than a runtime one, and `E_STORAGE_FATAL` when the
    /// city root's real location does not resolve.
    pub fn new(city_root: &Path, bound: ReadBound) -> Result<SearchTool, AxError> {
        Ok(SearchTool {
            city: chosen_path::CityRoot::anchor(city_root, "search")?,
            bound,
            meta: ToolMeta {
                name: ToolName::parse(Self::NAME)?,
                disclosure: "Searches the lines of files under one path for a regular \
                             expression in ripgrep's syntax, or with `glob` alone finds files by \
                             name. The line numbers it reports count from 1 and are the `offset` \
                             `read` continues from."
                    .to_owned(),
                params: Payload::of(&serde_json::json!({
                    "type": "object",
                    "properties": {
                        "pattern": { "type": "string", "description": "the regular expression \
                            to search the lines of files for, in ripgrep's syntax; leave it out \
                            and give `glob` to find files by name" },
                        "path": { "type": "string", "description": "a city-relative directory \
                            or file to search under; everything you may read when omitted" },
                        "glob": { "type": "string", "description": "only files this glob \
                            names, such as `*.rs` or `src/**/*.ts`; one without `/` is matched \
                            against the file name" },
                        "output_mode": { "type": "string",
                            "enum": ["files_with_matches", "content", "count"],
                            "description": "files_with_matches (default): the files that hold \
                            a match; content: every matching line with its line number; count: \
                            each such file with how many lines matched" },
                        "-i": { "type": "boolean", "description": "match without regard to \
                            case" },
                        "-B": { "type": "integer", "description": "lines to show before each \
                            match in content, 0 to 4" },
                        "-A": { "type": "integer", "description": "lines to show after each \
                            match in content, 0 to 4" },
                        "-C": { "type": "integer", "description": "lines to show before and \
                            after each match in content, 0 to 4" },
                        "context": { "type": "integer", "description": "the same as `-C`" },
                    },
                }))?,
                effect: Effect::Read,
                cost_tier: CostTier::Light,
                timeout: None,
                render: RenderIntent::Generic,
                temporal: Temporal::Timeless,
            },
        })
    }

    /// Where the walk starts, as an absolute path and as the city
    /// address that prefixes every hit it reports.
    fn start(&self, call: &ToolCall) -> Result<(PathBuf, String), AxError> {
        match call.args.as_map().get("path").and_then(Value::as_str) {
            None => Ok((self.city.written().to_path_buf(), String::new())),
            Some(asked) => {
                let spelled = chosen_path::within_city(self.city.written(), asked, "search")?;
                if spelled.is_empty() {
                    return Ok((self.city.written().to_path_buf(), String::new()));
                }
                let addr = chosen_path::admit(&spelled, "search", &*self.bound)?;
                let located = chosen_path::land(&self.city, &addr, "search", &*self.bound)?;
                let chosen_path::Located::Present(path) = located else {
                    return Err(AxError::failure(
                        AxCode::InvalidArgs,
                        "search",
                        format!("{asked} is not a place in this city"),
                    )
                    .with_recovery("name a path that exists, or leave `path` out to search all"));
                };
                Ok((path, addr.as_str().to_owned()))
            }
        }
    }

    /// Walks the subtree in name order, so the same city and the same
    /// question answer the same way twice. A file the predicate does not
    /// admit by name is passed over unopened.
    fn walk(&self, from: (PathBuf, String), looking: &Predicate) -> Findings {
        let root = from.1.clone();
        let mut found = Findings {
            hits: Vec::new(),
            spent: 0,
            unreadable: 0,
            unread: Vec::new(),
            stopped: None,
        };
        let mut pending = vec![Step::Visit(from.0, from.1)];
        while let Some(step) = pending.pop() {
            if found.stopped.is_some() {
                break;
            }
            let (path, rel) = match step {
                Step::Visit(path, rel) => (path, rel),
                Step::Unread { child, why } => {
                    found.could_not_look(&child, why);
                    continue;
                }
            };
            let path = match chosen_path::walked(&self.city, path, &rel, &*self.bound) {
                Ok(Walked::Directory(path)) => path,
                Ok(Walked::File(path)) => {
                    if looking.admits(&root, &rel) {
                        self.scan_file(&path, &rel, looking, &mut found);
                    }
                    continue;
                }
                Ok(Walked::Passed) => continue,
                Err(unresolved) => {
                    found.could_not_look(&rel, unresolved.to_string());
                    continue;
                }
            };
            match sorted_entries(&path) {
                Ok(entries) => {
                    // Reversed, because the stack hands back what was
                    // pushed last and the answer is in name order.
                    for name in entries.into_iter().rev() {
                        match admissible(&rel, &name, &*self.bound) {
                            Entry::Descend(child) => {
                                pending.push(Step::Visit(path.join(&name), child));
                            }
                            Entry::Unread { child, why } => {
                                pending.push(Step::Unread { child, why })
                            }
                            Entry::Passed => {}
                        }
                    }
                }
                Err(err) => found.could_not_look(&rel, format!("would not open: {err}")),
            }
        }
        found
    }

    /// Looks through one file, adding what it finds until the cap; a
    /// file found by name alone is listed without being opened.
    fn scan_file(&self, path: &Path, rel: &str, looking: &Predicate, found: &mut Findings) {
        let asked = match looking {
            Predicate::Names(_) => return found.list(Value::String(rel.to_owned())),
            Predicate::Lines(asked, _) => asked,
        };
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
        let mut matching = body.lines().filter(|line| asked.pattern.is_match(line));
        let listed = match asked.mode {
            Mode::Content => return Self::content_hits(&body, rel, asked, found),
            Mode::Files => matching.next().map(|_| Value::String(rel.to_owned())),
            Mode::Count => match matching.count() {
                0 => None,
                count => Some(serde_json::json!({ "path": rel, "count": count })),
            },
        };
        if let Some(entry) = listed {
            found.list(entry);
        }
    }

    /// Every hit in one file's text, with its context, until a cap.
    fn content_hits(body: &str, rel: &str, asked: &Lines, found: &mut Findings) {
        let lines: Vec<&str> = body.lines().collect();
        for (at, line) in lines.iter().enumerate() {
            if !asked.pattern.is_match(line) {
                continue;
            }
            if found.hits.len() >= MATCH_CAP {
                found.stopped = Some(Limit::Hits);
                return;
            }
            // Sixty-four hits is a bound on how many answers travel, not
            // on how large they are: nine context lines out of a
            // generated file are nine lines of whatever that file puts
            // on a line. The byte budget is the same ceiling `read`
            // keeps, and for the same reason - one result may not spend
            // the window a caller still has to think in.
            let (from, block) = context_block(&lines, at, asked);
            let budget = kernel::consts_policy::INTERVAL_CAP_BYTES;
            let fits = found.spent.saturating_add(block.len()) <= budget;
            if !fits && !found.hits.is_empty() {
                found.stopped = Some(Limit::Bytes);
                return;
            }
            // The first hit is held to the budget as well, cut at its
            // tail and marked: its line number is where `read` takes up
            // the rest, so nothing is lost by cutting it.
            let text = if fits {
                block
            } else {
                found.stopped = Some(Limit::Bytes);
                let keep = budget.saturating_sub(elision::marker_room(block.len()));
                elision::splice(&block, keep, block.len(), Elided::Tail).text
            };
            found.spent = found.spent.saturating_add(text.len());
            let mut hit = Map::new();
            hit.insert("path".to_owned(), Value::String(rel.to_owned()));
            hit.insert("line".to_owned(), Value::Number(at.into()));
            if from < at {
                // Where the context starts, which the drawn answer numbers
                // its first line from (`crate::rendering`).
                hit.insert("from".to_owned(), Value::Number(from.into()));
            }
            hit.insert("text".to_owned(), Value::String(text));
            found.hits.push(Value::Object(hit));
            if found.stopped.is_some() {
                return;
            }
        }
    }
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

impl Tool for SearchTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
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
        let looking = Predicate::asked_by(call)?;
        let found = self.walk(self.start(call)?, &looking);
        let mut out = Map::new();
        out.insert("count".to_owned(), Value::Number(found.hits.len().into()));
        out.insert("truncated".to_owned(), Value::Bool(found.stopped.is_some()));
        if let Some(limit) = found.stopped {
            out.insert("stopped".to_owned(), Value::String(limit.said()));
        }
        // What the walk could not open, rather than silence about it: a
        // search that reports nothing is a different answer from a
        // search that could not look.
        out.insert(
            "unreadable".to_owned(),
            Value::Number(found.unreadable.into()),
        );
        out.insert("unread".to_owned(), Value::Array(found.unread));
        out.insert(looking.key().to_owned(), Value::Array(found.hits));
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
