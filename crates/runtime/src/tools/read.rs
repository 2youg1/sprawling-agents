// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The read tool: one argument that is either a path this city lets a
//! model choose, or a name its reading room already admitted.
//!
//! **Two ways in, and the difference is who chose.** A path is chosen by
//! the model, so it is judged: it must parse as an address, it must not
//! reach a reserved subtree — the accounting, the configuration, the
//! rules and the history stay unreadable to the thing they govern — and
//! the read bound must let this run's building read it. A
//! catalog name was chosen by the person who wrote the building's
//! reading room, and admission happened when they wrote it; the skill it
//! resolves to may therefore sit in reserved space, because the answer to
//! "may this run see it" was given before the run existed.
//!
//! Without this tool the catalog could name a skill and never hand it
//! over, and every file the prompt asks an agent to consult had to be
//! reached by writing Python inside `exec` — which a city with no
//! sandbox and no shell cannot do at all.
//!
//! **An answer is an interval, and it says what it is part of.** Whole-
//! file reading made every long document an all-or-nothing call: the
//! kernel's own SPEC is fourteen hundred lines, and a read of it either
//! spent the window or was cut somewhere by the pipeline — usually
//! through the passage the caller wanted. So a call takes `offset` and
//! `limit`, at most 512 lines, and a truncated answer carries the total
//! and the offset to continue from, which is the number `search`
//! reports for a hit.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use kernel::{
    AxCode, AxError, CostTier, Effect, Payload, RenderIntent, Temporal, Tool, ToolCall, ToolMeta,
    ToolName, ToolOutcome,
};
use serde_json::{Map, Value};

mod locator;
mod miss;
mod package;

use super::chosen_path::{Located, ReadBound, real_location};
use crate::catalog::{Catalog, Expansion};
use miss::Floor;

/// The most lines one call may bring back, and the default: a default
/// below the cap would hide the cap from every caller that never sets
/// `limit`. A `u16`, so widening it to `usize` cannot fail.
const LINE_CAP: u16 = 512;

/// Where the answer to one call comes from.
enum Found {
    /// A place on disk, and the highest directory a miss there may list.
    File {
        at: Located,
        floor: Floor,
    },
    Text(String),
}

/// The interval of one answer: where it starts and how much of it there
/// is at most. `offset` stays the number the caller sent until it meets
/// the text it counts the lines of.
struct Interval {
    offset: u64,
    limit: u16,
}

/// What one interval took out of a document, and what the taking left.
///
/// `total_lines` and `next_offset` are present only when they say
/// something: a whole small file needs neither, and an offset past the
/// end gets the total without a `next_offset`, because handing one back
/// would invite a call that reads the same nothing again.
struct Taken {
    text: String,
    total_lines: Option<usize>,
    next_offset: Option<usize>,
}

impl Interval {
    /// # Errors
    /// `E_INVALID_ARGS` when `offset` or `limit` is not a whole number,
    /// or when `limit` is zero. A `limit` above the cap is served at the
    /// cap rather than refused: the answer carries the truth about what
    /// it left behind, so a refusal would only cost the caller a turn.
    fn asked_for(call: &ToolCall) -> Result<Interval, AxError> {
        let args = call.args.as_map();
        let whole = |field: &'static str| -> Result<Option<u64>, AxError> {
            match args.get(field) {
                None => Ok(None),
                Some(value) => value.as_u64().map(Some).ok_or_else(|| {
                    AxError::failure(
                        AxCode::InvalidArgs,
                        "read",
                        format!("`{field}` is not a whole number of lines"),
                    )
                    .with_recovery(
                        "pass `offset` and `limit` as non-negative integers, or leave them out \
                         to read from the start",
                    )
                }),
            }
        };
        let offset = whole("offset")?.unwrap_or(0);
        // Above the cap is served at the cap, and a number no `u16`
        // holds is above it.
        let limit = whole("limit")?.map_or(LINE_CAP, |asked| {
            u16::try_from(asked).map_or(LINE_CAP, |fits| fits.min(LINE_CAP))
        });
        if limit == 0 {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "read",
                "`limit` is zero, which asks for no lines at all",
            )
            .with_recovery("pass a `limit` from 1 to 512, or leave it out"));
        }
        Ok(Interval { offset, limit })
    }

    /// Cuts on line ends rather than on lines: every line travels with
    /// its own terminator, so reading a whole short file returns the
    /// bytes the file holds and `bytes` keeps the meaning it had.
    ///
    /// **Two ceilings, and the tighter one decides.** 512 lines bounds
    /// how much structure an answer carries;
    /// [`INTERVAL_CAP_BYTES`](kernel::consts_policy::INTERVAL_CAP_BYTES)
    /// bounds how much of the window it spends, because a line is about
    /// forty bytes in ordinary source and a whole minified bundle in a
    /// generated one. The byte ceiling is applied here rather than
    /// downstream for one reason: `next_offset` is how a caller
    /// continues, so a cut made after this function had already reported
    /// one would leave that number pointing past lines nobody delivered,
    /// and the gap would be silent.
    fn cut(&self, text: &str) -> Taken {
        let lines: Vec<&str> = text.split_inclusive('\n').collect();
        let total = lines.len();
        // An offset no `usize` holds is past the end of every text this
        // machine can hold, which is the answer the second arm gives.
        let start = match usize::try_from(self.offset) {
            Ok(start) if start < total => start,
            Ok(_) | Err(_) => {
                return Taken {
                    text: String::new(),
                    total_lines: Some(total),
                    next_offset: None,
                };
            }
        };
        let ceiling = start.saturating_add(usize::from(self.limit)).min(total);
        let budget = kernel::consts_policy::INTERVAL_CAP_BYTES;
        let mut end = start;
        let mut spent = 0usize;
        while end < ceiling {
            let Some(line) = lines.get(end) else { break };
            let after = spent.saturating_add(line.len());
            // The first line travels however long it is. A call that
            // returned nothing would hand back the offset it was given,
            // and the caller would ask the same question forever.
            if after > budget && end > start {
                break;
            }
            spent = after;
            end = end.saturating_add(1);
        }
        let taken = lines.get(start..end).unwrap_or_default().concat();
        if end < total {
            Taken {
                text: taken,
                total_lines: Some(total),
                next_offset: Some(end),
            }
        } else {
            Taken {
                text: taken,
                total_lines: None,
                next_offset: None,
            }
        }
    }
}

/// What a read answers with, and what it refuses.
pub struct ReadTool {
    city_root: PathBuf,
    /// The same catalog the model was shown, shared: a copy would be a
    /// second authority on what this run may open, and copies drift.
    catalog: Arc<Mutex<Catalog>>,
    /// What this run's building may read, asked of every path the model
    /// chooses and of the building a Locator's bytes belong to.
    bound: ReadBound,
    /// Where this city keeps content blocks: a run may write in a
    /// worktree that holds no store of its own.
    block_store: PathBuf,
    meta: ToolMeta,
}

impl ReadTool {
    /// # Errors
    /// Propagates a malformed parameter schema, which is a build-time
    /// defect rather than a runtime one.
    pub fn new(
        city_root: &Path,
        catalog: Arc<Mutex<Catalog>>,
        bound: ReadBound,
        block_store: &Path,
    ) -> Result<ReadTool, AxError> {
        let mut params = Map::new();
        params.insert("type".to_owned(), Value::String("object".to_owned()));
        let mut properties = Map::new();
        let mut spec = Map::new();
        spec.insert("type".to_owned(), Value::String("string".to_owned()));
        spec.insert(
            "description".to_owned(),
            Value::String(
                "a file path relative to the city root, or the name of a catalog entry".to_owned(),
            ),
        );
        properties.insert("path".to_owned(), Value::Object(spec));
        for (name, description) in [
            (
                "offset",
                "the first line to return, counted from 0; default 0",
            ),
            (
                "limit",
                "how many lines to return, at most 512, which is also the default",
            ),
        ] {
            let mut spec = Map::new();
            spec.insert("type".to_owned(), Value::String("integer".to_owned()));
            spec.insert(
                "description".to_owned(),
                Value::String(description.to_owned()),
            );
            properties.insert(name.to_owned(), Value::Object(spec));
        }
        params.insert("properties".to_owned(), Value::Object(properties));
        params.insert(
            "required".to_owned(),
            Value::Array(vec![Value::String("path".to_owned())]),
        );
        Ok(ReadTool {
            city_root: city_root.to_path_buf(),
            catalog,
            bound,
            block_store: block_store.to_path_buf(),
            meta: ToolMeta {
                name: ToolName::parse("read")?,
                disclosure: "Read a file by its path, or a skill by the name the catalog lists \
                             it under. A truncated answer states the total and the offset to \
                             continue from. Reading a directory or a missing file is refused."
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

    /// Where one argument points, or why it points nowhere.
    ///
    /// Catalog entries are tried first: a building that admits a skill
    /// called `review` has said what that word means here, and a file
    /// that happens to share the name must not be able to shadow it.
    fn resolve(&self, asked: &str) -> Result<Found, AxError> {
        if let Some(read) =
            locator::open_locator(asked, &self.city_root, &self.block_store, &*self.bound)
        {
            return read.map(Found::Text);
        }
        let catalog = self.catalog.lock().map_err(|_| {
            AxError::failure(
                AxCode::StorageFatal,
                "read",
                "the catalog was left locked by a thread that died",
            )
            .with_recovery(
                "end this run and resume it: the catalog cannot be trusted again inside a \
                 process where a thread died holding it",
            )
        })?;
        if let Some(expansion) = catalog.expand(asked) {
            return match expansion {
                Expansion::Skill { addr, .. } => {
                    let addr = kernel::Address::parse(&addr).map_err(|err| {
                        AxError::failure(
                            AxCode::ConfigInvalid,
                            "read",
                            format!("{asked} is shelved at {}", err.subject()),
                        )
                        .with_recovery(
                            "this building's reading room names a skill the city cannot address; \
                             a person has to fix the shelf",
                        )
                    })?;
                    Ok(Found::File {
                        at: real_location(
                            &(addr.as_str().split('/'))
                                .fold(self.city_root.clone(), |at, part| at.join(part)),
                            "read",
                            asked,
                        )?,
                        floor: Floor::Document,
                    })
                }
                // The catalog's own second level. The prompt carries one
                // line per entry, and this is what that line stood for,
                // so it is handed over rather than refused.
                Expansion::Said { text } => Ok(Found::Text(text)),
            };
        }
        if let Some(inside) = package::open_in_package(&catalog, &self.city_root, asked) {
            return inside;
        }
        drop(catalog);
        // The judgement every model-chosen path gets, in the one place
        // it is written. `search` asks the same function the same
        // question, so what is reserved and what is closed have one
        // answer each.
        let addr = super::chosen_path::admit(asked, "read", &*self.bound)?;
        Ok(Found::File {
            at: super::chosen_path::land(&self.city_root, &addr, "read", &*self.bound)?,
            floor: Floor::of_address(&self.city_root, &addr),
        })
    }
}

impl Tool for ReadTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "read",
                format!("call routed to the wrong tool: {}", call.name.as_str()),
            )
            .with_recovery(format!(
                "call `{}`, the name this tool answers to",
                self.meta.name.as_str()
            )));
        }
        let asked = call
            .args
            .as_map()
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "read",
                    "missing string argument `path`",
                )
                .with_recovery("pass one string: a city-relative path, or a catalog name")
            })?;
        let text = match self.resolve(asked)? {
            Found::Text(text) => text,
            Found::File { at, floor } => miss::text_at(asked, at, &floor)?,
        };
        let mut out = Map::new();
        out.insert("path".to_owned(), Value::String(asked.to_owned()));
        let interval = Interval::asked_for(call)?;
        let taken = interval.cut(&text);
        if let Some(total) = taken.total_lines {
            out.insert("total_lines".to_owned(), Value::Number(total.into()));
        }
        if let Some(next) = taken.next_offset {
            out.insert("next_offset".to_owned(), Value::Number(next.into()));
        }
        // The count the model needs to decide whether it has the whole
        // thing: a result the pipeline shortened says so in its own
        // envelope, and this is what it was shortened from.
        out.insert("bytes".to_owned(), Value::Number(taken.text.len().into()));
        out.insert("text".to_owned(), Value::String(taken.text));
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
