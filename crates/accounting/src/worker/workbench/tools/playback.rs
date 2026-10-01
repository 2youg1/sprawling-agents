// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `playback`: a resident exports a stretch of the history its building
//! may read, as a bundle or a playback page, and checks an export of its
//! building against the city (sprawling-SPEC.md 8-132).
//!
//! The reader is bound when the bench is laid out: the building the run
//! stands in. No argument names a reader, and an argument this tool does
//! not know is refused, so a resident cannot widen what it reads.
//! Exports land under the city's playback exports, in the building's own
//! directory, through the one landing every export takes
//! (accounting-SPEC.md 8-13).

use std::path::{Path, PathBuf};

use kernel::layout::CityLayout;
use kernel::{
    Address, AxCode, AxError, CostTier, Effect, Payload, RenderIntent, RunId, Seq, Temporal, Tool,
    ToolCall, ToolMeta, ToolName, ToolOutcome, Writes,
};
use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::playback::{
    Asked, BUNDLE_BLOCK, City, Cutoff, PAGE_MAX_BYTES, Place, Reader, Request, Selection,
};
use crate::worker::workbench::{Laying, Site};

/// What every refusal of this tool names as the action that failed.
const ACTION: &str = "playback";

/// The longest export name.
const NAME_MAX: usize = 64;

/// The tool: the city it reads, and the building whose resident reads it.
pub(super) struct PlaybackTool {
    city_root: PathBuf,
    building: Address,
    meta: ToolMeta,
}

/// The arguments of `export`, besides the action.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Export {
    name: String,
    from: Option<u64>,
    through: Option<u64>,
    run: Option<String>,
    building: Option<String>,
    page: Option<String>,
}

/// The arguments of `check`, besides the action.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Check {
    file: String,
}

/// The two kinds of export file, by what they hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Bundle,
    Page,
}

impl Kind {
    fn extension(self) -> &'static str {
        match self {
            Kind::Bundle => "json",
            Kind::Page => "html",
        }
    }
}

impl Laying {
    /// The playback tool for a run in `room`, answering as the building
    /// of `site` and writing into this city's playback exports.
    ///
    /// # Errors
    /// Propagates [`PlaybackTool::new`]'s refusal.
    pub(super) fn playback_tool(
        &self,
        site: &Site,
        room: &Address,
    ) -> Result<PlaybackTool, AxError> {
        PlaybackTool::new(&self.city_root, site.building.addr().clone(), room.clone())
    }
}

impl PlaybackTool {
    /// The tool for a run in `room`, whose building is `building`.
    ///
    /// # Errors
    /// Refuses a name or a parameter schema that does not build, which
    /// the literals below cannot produce.
    fn new(city_root: &Path, building: Address, room: Address) -> Result<PlaybackTool, AxError> {
        let params = json!({
            "type": "object",
            "properties": {
                "action": {"type": "string", "enum": ["export", "check"]},
                "name": {
                    "type": "string",
                    "description": "export: the file's name, 1 to 64 letters, digits, - or _",
                },
                "from": {"type": "integer", "description": "export: the first seq, included"},
                "through": {"type": "integer", "description": "export: the last seq, included"},
                "run": {"type": "string", "description": "export: only this run's lines"},
                "building": {
                    "type": "string",
                    "description": "export: only lines addressed within this building",
                },
                "page": {
                    "type": "string",
                    "description": format!(
                        "export: an HTML template holding {BUNDLE_BLOCK} once; the bundle is \
                         put inside it and the page is checked before it lands"
                    ),
                },
                "file": {
                    "type": "string",
                    "description": "check: an export of your building, <name>.json or <name>.html",
                },
            },
            "required": ["action"],
        });
        let Value::Object(params) = params else {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                ACTION,
                "the parameter schema is not an object",
            )
            .with_recovery("report this against accounting::worker::workbench::tools::playback"));
        };
        Ok(PlaybackTool {
            city_root: city_root.to_path_buf(),
            building,
            meta: ToolMeta {
                name: ToolName::parse(ACTION)?,
                disclosure: "Look back at this city's history as your building may read it. \
                             `export` writes a playback bundle, or a playback page when you \
                             pass `page`, into your building's playback exports and never over \
                             an earlier one; `check` checks one of those exports against the \
                             city and reports each item on its own. The playback skill says how \
                             to write a page."
                    .to_owned(),
                params: Payload::new(params)?,
                // An export writes a file. The effect is the registration's,
                // so `check` rides the same door and is never run ahead.
                effect: Effect::Write { domain: room },
                // One strict pass over the whole ledger.
                cost_tier: CostTier::Heavy,
                timeout: None,
                render: RenderIntent::Generic,
                temporal: Temporal::Timeless,
            },
        })
    }

    /// The building's own directory of exports.
    fn exports(&self) -> PathBuf {
        self.building.as_str().split('/').fold(
            CityLayout::new(&self.city_root).playback_exports(),
            |path, segment| path.join(segment),
        )
    }

    fn export(&self, asked: Export) -> Result<Value, AxError> {
        let kind = if asked.page.is_some() {
            Kind::Page
        } else {
            Kind::Bundle
        };
        let file = format!("{}.{}", named(&asked.name)?, kind.extension());
        let selection = Selection::new(
            asked.from.map(Seq::new),
            asked.through.map(Seq::new),
            asked.run.as_deref().map(run_of).transpose()?,
            asked.building.as_deref().map(building_of).transpose()?,
        )?;
        let request = Request {
            selection,
            reader: Reader::Resident(self.building.clone()),
            cutoff: Cutoff::Latest,
        };
        let bundle = crate::playback::export(&self.city_root, &request)?;
        let bytes = match &asked.page {
            Some(template) => crate::playback::embed(template.as_bytes(), &bundle)?,
            None => bundle.bytes().to_vec(),
        };
        crate::playback::land(
            Place::Exports {
                city_root: &self.city_root,
                file: &self.exports().join(&file),
            },
            &bytes,
        )?;
        let mut out = Map::new();
        out.insert("file".to_owned(), file.into());
        out.insert("digest".to_owned(), bundle.digest().to_string().into());
        out.insert("events".to_owned(), bundle.events().to_string().into());
        if kind == Kind::Page {
            for item in ["structure", "offline"] {
                out.insert(item.to_owned(), json!({"status": "passed"}));
            }
        }
        Ok(Value::Object(out))
    }

    fn check(&self, asked: &Check) -> Result<Value, AxError> {
        let (stem, extension) = asked
            .file
            .rsplit_once('.')
            .filter(|(_, extension)| ["json", "html"].contains(extension))
            .ok_or_else(|| refused(format!("`{}` is not an export's file name", asked.file)))?;
        let path = self.exports().join(format!("{}.{extension}", named(stem)?));
        let cleared =
            storage::WriteTarget::within("check a playback export", &self.city_root, &path)
                .map_err(storage::StorageError::into_ax)?;
        let bytes = read_export(cleared.as_path(), &asked.file)?;
        Ok(crate::playback::check(
            &bytes,
            &Asked {
                city: Some(City {
                    root: &self.city_root,
                    reader: Reader::Resident(self.building.clone()),
                }),
                ..Asked::default()
            },
        )
        .line())
    }
}

impl Tool for PlaybackTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                ACTION,
                format!("call routed to the wrong tool: {}", call.name.as_str()),
            )
            .with_recovery(format!(
                "call `{}`, the name this tool answers to",
                self.meta.name.as_str()
            )));
        }
        let mut args = call.args.as_map().clone();
        let answer = match args.remove("action") {
            Some(Value::String(action)) if action == "export" => {
                self.export(arguments::<Export>(args)?)?
            }
            Some(Value::String(action)) if action == "check" => {
                self.check(&arguments::<Check>(args)?)?
            }
            Some(_) | None => {
                return Err(refused(
                    "`action` is neither `export` nor `check`".to_owned(),
                ));
            }
        };
        let Value::Object(result) = answer else {
            return Err(refused("the answer is not an object".to_owned()));
        };
        Ok(ToolOutcome {
            result: Payload::new(result)?,
            attachments: Vec::new(),
        })
    }

    /// An export lands in the city's reserved subtree, outside the tree
    /// a checkpoint stages, and a check writes nothing.
    fn writes(&self, _call: &ToolCall) -> Writes {
        Writes::Nothing
    }
}

/// One action's arguments, every field known.
fn arguments<T: for<'de> Deserialize<'de>>(args: Map<String, Value>) -> Result<T, AxError> {
    serde_json::from_value(Value::Object(args)).map_err(|err| {
        AxError::failure(AxCode::InvalidArgs, ACTION, err.to_string()).with_recovery(
            "pass only the arguments this action takes; the reader is your building and is not \
             an argument",
        )
    })
}

/// `name`, when it is a name an export may have.
fn named(name: &str) -> Result<&str, AxError> {
    let mut chars = name.chars();
    let lawful = chars
        .next()
        .is_some_and(|first| first.is_ascii_alphanumeric())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && name.len() <= NAME_MAX;
    if lawful {
        Ok(name)
    } else {
        Err(refused(format!(
            "`{name}` is not an export name: 1 to {NAME_MAX} ASCII letters, digits, - or _, \
             starting with a letter or digit"
        )))
    }
}

fn run_of(raw: &str) -> Result<RunId, AxError> {
    RunId::parse(raw).map_err(|err| refused(format!("`{raw}` is not a run id: {}", err.subject())))
}

fn building_of(raw: &str) -> Result<Address, AxError> {
    Address::parse(raw).map_err(|err| {
        refused(format!(
            "`{raw}` is not a building's address: {}",
            err.subject()
        ))
    })
}

/// The bytes of an export, refusing one that is not there or too large.
fn read_export(path: &Path, file: &str) -> Result<Vec<u8>, AxError> {
    let size = match std::fs::metadata(path) {
        Ok(meta) => meta.len(),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(refused(format!(
                "`{file}` is not among your building's exports"
            )));
        }
        Err(err) => return Err(unreadable(file, &err)),
    };
    if !usize::try_from(size).is_ok_and(|size| size <= PAGE_MAX_BYTES) {
        return Err(refused(format!("`{file}` is over {PAGE_MAX_BYTES} bytes")));
    }
    std::fs::read(path).map_err(|err| unreadable(file, &err))
}

fn refused(subject: String) -> AxError {
    AxError::failure(AxCode::InvalidArgs, ACTION, subject).with_recovery(
        "export with `name` and an optional range, run, building or page; check with `file`, \
         the name an export answered with",
    )
}

fn unreadable(file: &str, err: &std::io::Error) -> AxError {
    AxError::failure(AxCode::StorageFatal, ACTION, format!("{file}: {err}"))
        .with_recovery("a person has to make the city's reserved subtree readable")
}

#[cfg(test)]
mod tests;
