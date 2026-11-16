// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;
use std::sync::{Arc, Mutex};

use kernel::{
    AxCode, AxError, B3Hash, CostTier, Effect, ImageRef, Payload, RenderIntent, Temporal, TimeMs,
    Tool, ToolCall, ToolDef, ToolMeta, ToolName, ToolOutcome,
};
use serde_json::{Map, Value};

mod engine;
pub(super) mod locator;
mod miss;
mod package;

mod interval;
mod rules;

use super::BoundReader;
use super::chosen_path::{CityRoot, ReadBound};
use super::shown::{Claim, Observation, ReadShown, Span};
use super::stamp::ReadStamps;
use crate::catalog::{Catalog, Expansion};

pub type ReadClock = Arc<dyn Fn() -> Result<TimeMs, AxError> + Send + Sync>;

use interval::{Found, Interval};
pub use rules::{RuleCandidate, RuleCandidates, RuleContext};

pub struct ReadTool {
    catalog: Arc<Mutex<Catalog>>,

    reader: BoundReader,

    picture_origin: Option<storage::BlockOrigin>,

    city: CityRoot,

    seen: ReadStamps,
    shown: ReadShown,
    clock: Option<ReadClock>,
    rules: Option<RuleCandidates>,
    meta: ToolMeta,
}

impl ReadTool {
    pub const NAME: &'static str = "read";

    pub const FILE_PATH: &'static str = "file_path";

    pub const RULE_CONTEXT: &'static str = "rule_context";

    pub fn definition() -> Result<ToolDef, AxError> {
        let mut params = Map::new();
        params.insert("type".to_owned(), Value::String("object".to_owned()));
        let mut properties = Map::new();
        let mut spec = Map::new();
        spec.insert("type".to_owned(), Value::String("string".to_owned()));
        spec.insert(
            "description".to_owned(),
            Value::String(
                "the file to read, relative to the city root, or the name of a catalog entry"
                    .to_owned(),
            ),
        );
        properties.insert(Self::FILE_PATH.to_owned(), Value::Object(spec));
        for (name, description) in [
            (
                "offset",
                "the line number to start reading from, counting from 1; default 1",
            ),
            (
                "limit",
                "the number of lines to read, at most 512, which is also the default",
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
            Value::Array(vec![Value::String(Self::FILE_PATH.to_owned())]),
        );
        Ok(ToolDef {
            name: ToolName::parse(Self::NAME)?,
            description: "Reads a file by its path, or a skill by the name the catalog \
                          lists it under, with line numbers counted from 1. A truncated \
                          answer states the total and the line to continue from. PNG, JPEG, \
                          WebP and GIF come back as picture attachments within the picture \
                          size limit. Reading a directory or a missing file is refused."
                .to_owned(),
            input_schema: Payload::new(params)?,
        })
    }

    pub fn new(
        city_root: &Path,
        catalog: Arc<Mutex<Catalog>>,
        bound: ReadBound,
        block_store: &Path,
    ) -> Result<ReadTool, AxError> {
        let defined = Self::definition()?;
        Ok(ReadTool {
            catalog,
            reader: BoundReader::new(city_root, bound, block_store),
            picture_origin: None,
            city: CityRoot::anchor(city_root, Self::NAME)?,
            seen: ReadStamps::default(),
            shown: ReadShown::default(),
            clock: None,
            rules: None,
            meta: ToolMeta {
                name: defined.name,
                disclosure: defined.description,
                params: defined.input_schema,
                effect: Effect::Read,
                cost_tier: CostTier::Light,
                timeout: None,
                render: RenderIntent::Generic,
                temporal: Temporal::Timeless,
            },
        })
    }

    fn resolve(&self, asked: &str) -> Result<Found, AxError> {
        if let Some(read) = locator::open_locator(asked, &self.reader, "read") {
            let (_, bytes) = read?;
            if let Some(picture) = crate::pipeline::connector::read_picture(&bytes)? {
                return Ok(Found::Picture { picture, bytes });
            }
            return String::from_utf8(bytes).map(Found::Text).map_err(|_| {
                AxError::failure(AxCode::InvalidArgs, "read", format!("{asked} is not text"))
                    .with_recovery("read hands back text; these bytes are not UTF-8")
            });
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
                Expansion::Skill { manifest } => {
                    package::open_document(&self.reader.block_store, &manifest, asked)
                }

                Expansion::Said { text } => Ok(Found::Text(text)),
            };
        }
        if let Some(inside) = package::open_in_package(&catalog, &self.reader.block_store, asked) {
            return inside;
        }
        drop(catalog);

        let spelled = super::chosen_path::within_city(self.city.written(), asked, "read")?;
        let addr = super::chosen_path::admit(&spelled, "read", &*self.reader.bound)?;
        Ok(Found::Chosen(addr))
    }

    #[must_use]
    pub fn noting(self, seen: ReadStamps) -> ReadTool {
        ReadTool { seen, ..self }
    }

    #[must_use]
    pub fn recalling(self, shown: ReadShown) -> ReadTool {
        ReadTool { shown, ..self }
    }

    #[must_use]
    pub fn clocking(self, clock: ReadClock) -> ReadTool {
        ReadTool {
            clock: Some(clock),
            ..self
        }
    }

    #[must_use]
    pub fn storing_pictures(self, origin: storage::BlockOrigin) -> ReadTool {
        ReadTool {
            picture_origin: Some(origin),
            ..self
        }
    }

    #[must_use]
    pub fn ruling(self, candidates: RuleCandidates) -> ReadTool {
        ReadTool {
            rules: Some(candidates),
            ..self
        }
    }

    fn picture_at(&self, asked: &str) -> Result<Option<(ImageRef, Vec<u8>)>, AxError> {
        let mut opened = self.reader.open(asked, Self::NAME)?;
        let bytes = kernel::consts_policy::IMAGE_MAX_BYTES
            .read_from(&mut opened)
            .map_err(|err| {
                AxError::failure(AxCode::StorageFatal, Self::NAME, format!("{asked}: {err}"))
                    .with_recovery("the User has to make the file readable")
            })?;
        let picture = crate::pipeline::connector::read_picture(&bytes)?;
        Ok(picture.map(|picture| (picture, bytes)))
    }

    fn present_picture(
        &self,
        asked: &str,
        picture: ImageRef,
        bytes: &[u8],
    ) -> Result<ToolOutcome, AxError> {
        let origin = self.picture_origin.as_ref().ok_or_else(|| {
            AxError::failure(
                AxCode::ConfigInvalid,
                "read a picture",
                format!("{asked}: this read tool has no picture origin"),
            )
            .with_recovery("construct the read tool with `storing_pictures` for this run")
        })?;
        storage::Cas::open(&self.reader.block_store)
            .map_err(storage::StorageError::into_ax)?
            .put_for(bytes, origin)
            .map_err(storage::StorageError::into_ax)?;
        let text = crate::pipeline::connector::picture_line(&picture);
        let mut out = Map::new();
        out.insert("path".to_owned(), Value::String(asked.to_owned()));
        out.insert("bytes".to_owned(), Value::from(text.len()));
        out.insert("text".to_owned(), Value::String(text));
        Ok(ToolOutcome {
            result: Payload::new(out)?,
            attachments: vec![picture],
        })
    }
}

impl Tool for ReadTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        self.shown.discard(&call.id);
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
            .get(Self::FILE_PATH)
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "read",
                    format!("missing string argument `{}`", Self::FILE_PATH),
                )
                .with_recovery(format!(
                    "pass `{}`: a city-relative path, or a catalog name; `offset` and `limit` \
                     are optional",
                    Self::FILE_PATH
                ))
            })?;
        let interval = Interval::asked_for(call)?;
        let want = Span {
            offset: interval.offset,
            lines: u64::from(interval.limit),
        };
        let mut out = Map::new();
        out.insert("path".to_owned(), Value::String(asked.to_owned()));
        let mut looked = None;
        let mut opened = None;
        let mut repeated = None;
        let mut digest = None;
        let sampled = self.clock.as_ref().and_then(|clock| clock().ok());
        let resolved = match self.resolve(asked)? {
            Found::Chosen(addr) if image::ImageFormat::from_path(asked).is_ok() => {
                match self.picture_at(asked)? {
                    Some((picture, bytes)) => Found::Picture { picture, bytes },
                    None => Found::Chosen(addr),
                }
            }
            other @ Found::Chosen(_) | other @ Found::Text(_) | other @ Found::Picture { .. } => {
                other
            }
        };
        let text = match resolved {
            Found::Picture { picture, bytes } => {
                return self.present_picture(asked, picture, &bytes);
            }
            Found::Text(text) => text,
            Found::Chosen(addr) => match engine::text_of(engine::Request {
                city: &self.city,
                addr: &addr,
                asked,
                bound: &*self.reader.bound,
                shown: &self.shown,
                want,
            })? {
                engine::Looked::Text {
                    text,
                    before,
                    after,
                    addr: landed,
                } => {
                    let racy = sampled.is_none_or(|now| before.racy_at(now));
                    digest = racy.then(|| B3Hash::digest(text.as_bytes()));
                    looked = Some((addr.clone(), before, after));
                    opened = Some((addr, landed));
                    text
                }
                engine::Looked::Unchanged {
                    stamp,
                    addr: landed,
                    answer,
                } => {
                    looked = Some((addr.clone(), stamp.clone(), stamp.clone()));
                    opened = Some((addr, landed));
                    repeated = Some(answer);
                    String::new()
                }
            },
        };

        if interval.offset != 0 {
            out.insert("offset".to_owned(), Value::Number(interval.offset.into()));
        }
        let taken = repeated.is_none().then(|| interval.cut(&text));
        if self.rules.is_some()
            && let Some((admitted, landed)) = opened
        {
            out.insert(
                Self::RULE_CONTEXT.to_owned(),
                RuleContext { admitted, landed }.into_json(),
            );
        }

        let windowed = ["offset", "limit"]
            .iter()
            .any(|key| call.args.as_map().contains_key(*key));
        let complete = repeated.is_some()
            || taken
                .as_ref()
                .is_some_and(|taken| taken.next_offset.is_none());
        if let Some((addr, before, after)) = looked
            && (windowed || complete)
        {
            if let Some(taken) = &taken
                && before == after
            {
                let total = u64::try_from(taken.total).unwrap_or(u64::MAX);
                let lines = u64::try_from(taken.lines).unwrap_or(u64::MAX);
                let observed = Observation::new(total, taken.text.len(), digest);
                let claim = if windowed {
                    Claim::window(
                        addr.clone(),
                        before.clone(),
                        Span {
                            offset: interval.offset,
                            lines,
                        },
                        observed,
                    )
                } else {
                    Claim::whole(addr.clone(), before.clone(), observed)
                };
                self.shown.stage(&call.id, claim);
            }
            self.seen.look(&addr, Some(before), Some(after));
        }
        if let Some(answer) = repeated {
            out.insert("unchanged".to_owned(), Value::Bool(true));
            out.insert("offset".to_owned(), Value::Number(interval.offset.into()));
            out.insert("lines".to_owned(), Value::Number(answer.lines.into()));
            out.insert(
                "total_lines".to_owned(),
                Value::Number(answer.total_lines.into()),
            );
        } else if let Some(taken) = taken {
            if let Some(total) = taken.total_lines {
                out.insert("total_lines".to_owned(), Value::Number(total.into()));
            }
            if let Some(next) = taken.next_offset {
                out.insert("next_offset".to_owned(), Value::Number(next.into()));
            }

            out.insert("bytes".to_owned(), Value::Number(taken.text.len().into()));
            out.insert("text".to_owned(), Value::String(taken.text));
        }
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
    clippy::permissions_set_readonly_false,
    reason = "test code, including a deliberately corrupted writable CAS object"
)]
mod tests;
