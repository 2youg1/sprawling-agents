// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use kernel::{Address, AxCode, AxError, B3Hash, RESERVED_PREFIX};

use crate::defaults::Shipped;
use crate::spine_files::CITY_FILE;
use crate::spine_files::hall::{CLERK_FILE, MAYOR_FILE};

const MODELS_DIR: &str = "models";

const NOTE_EXT: &str = "md";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptDocument {
    Shipped(Shipped),

    BuildingCity {
        building: Address,
    },

    ModelNote {
        building: Option<Address>,
        provider: String,
        model: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptStep {
    Own,

    Inherit,

    Restore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PromptWritten {
    pub bytes: usize,
    pub removed: bool,
}

impl PromptDocument {
    #[must_use]
    pub fn of(at: &Address) -> Option<PromptDocument> {
        let parts: Vec<&str> = at.as_str().split('/').collect();
        match parts.as_slice() {
            [CITY_FILE] => Some(PromptDocument::Shipped(Shipped::City)),
            [RESERVED_PREFIX, MAYOR_FILE] => Some(PromptDocument::Shipped(Shipped::Mayor)),
            [RESERVED_PREFIX, CLERK_FILE] => Some(PromptDocument::Shipped(Shipped::Clerk)),
            [building, RESERVED_PREFIX, CITY_FILE] => Some(PromptDocument::BuildingCity {
                building: a_building(building)?,
            }),
            [RESERVED_PREFIX, MODELS_DIR, provider, note @ ..] => note_of(None, provider, note),
            [building, RESERVED_PREFIX, MODELS_DIR, provider, note @ ..] => {
                note_of(Some(a_building(building)?), provider, note)
            }
            _ => None,
        }
    }

    pub fn at(&self) -> Result<Address, AxError> {
        let spelled = match self {
            PromptDocument::Shipped(Shipped::City) => CITY_FILE.to_owned(),
            PromptDocument::Shipped(Shipped::Mayor) => format!("{RESERVED_PREFIX}/{MAYOR_FILE}"),
            PromptDocument::Shipped(Shipped::Clerk) => format!("{RESERVED_PREFIX}/{CLERK_FILE}"),
            PromptDocument::BuildingCity { building } => {
                format!("{}/{RESERVED_PREFIX}/{CITY_FILE}", building.as_str())
            }
            PromptDocument::ModelNote {
                building,
                provider,
                model,
            } => note_spelling(building.as_ref(), provider, model),
        };
        Address::parse(&spelled)
    }

    pub fn path(&self, city_root: &Path) -> Result<PathBuf, AxError> {
        Ok(city_root.join(self.at()?.as_str()))
    }

    pub fn admits(&self, text: &[u8]) -> Result<(), AxError> {
        match self {
            PromptDocument::Shipped(Shipped::Mayor) => {
                let text = std::str::from_utf8(text).map_err(|err| {
                    AxError::failure(
                        AxCode::InvalidArgs,
                        "save the Mayor's persona",
                        err.to_string(),
                    )
                    .with_recovery("save the persona as UTF-8 text")
                })?;
                crate::identity::readable(crate::governed::Governed::Mayor, text)
            }
            PromptDocument::Shipped(Shipped::City | Shipped::Clerk)
            | PromptDocument::BuildingCity { .. }
            | PromptDocument::ModelNote { .. } => Ok(()),
        }
    }

    fn farther(&self) -> Option<PromptDocument> {
        match self {
            PromptDocument::BuildingCity { .. } => Some(PromptDocument::Shipped(Shipped::City)),
            PromptDocument::ModelNote {
                building: Some(_),
                provider,
                model,
            } => Some(PromptDocument::ModelNote {
                building: None,
                provider: provider.clone(),
                model: model.clone(),
            }),
            PromptDocument::Shipped(_) | PromptDocument::ModelNote { building: None, .. } => None,
        }
    }
}

#[must_use]
pub fn model_note_at(building: Option<&Address>, provider: &str, model: &str) -> Option<Address> {
    Address::parse(&note_spelling(building, provider, model)).ok()
}

pub fn model_notes(
    city_root: &Path,
    building: Option<&Address>,
) -> Result<Vec<(String, String)>, AxError> {
    let mut root = city_root.to_path_buf();
    if let Some(building) = building {
        root.push(building.as_str());
    }
    root.push(RESERVED_PREFIX);
    root.push(MODELS_DIR);
    let mut notes = Vec::new();
    for provider in names_in(&root)? {
        for file in files_under(&root.join(&provider), &mut Vec::new())? {
            let parts: Vec<&str> = file.iter().map(String::as_str).collect();
            if let Some(PromptDocument::ModelNote {
                provider, model, ..
            }) = note_of(None, &provider, &parts)
            {
                notes.push((provider, model));
            }
        }
    }
    notes.sort();
    Ok(notes)
}

fn files_under(dir: &Path, above: &mut Vec<PathBuf>) -> Result<Vec<Vec<String>>, AxError> {
    let resolved = match std::fs::canonicalize(dir) {
        Ok(resolved) => resolved,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(storage(dir, &err)),
    };
    if above.contains(&resolved) {
        return Ok(Vec::new());
    }
    let entries = std::fs::read_dir(dir).map_err(|err| storage(dir, &err))?;
    above.push(resolved);
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| storage(dir, &err))?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let path = entry.path();
        let kind = match std::fs::metadata(&path) {
            Ok(meta) => meta.file_type(),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => return Err(storage(&path, &err)),
        };
        if kind.is_dir() {
            for mut below in files_under(&path, above)? {
                below.insert(0, name.clone());
                files.push(below);
            }
        } else if kind.is_file() {
            files.push(vec![name]);
        }
    }
    above.pop();
    Ok(files)
}

pub fn put_prompt(
    city_root: &Path,
    doc: &PromptDocument,
    base: B3Hash,
    step: PromptStep,
) -> Result<PromptWritten, AxError> {
    let path = doc.path(city_root)?;
    match (step, doc) {
        (PromptStep::Restore, PromptDocument::Shipped(shipped)) => {
            crate::defaults::restore(city_root, *shipped, base).map(|bytes| PromptWritten {
                bytes,
                removed: false,
            })
        }
        (
            PromptStep::Own,
            PromptDocument::BuildingCity { .. } | PromptDocument::ModelNote { .. },
        ) => {
            let start = match doc.farther() {
                Some(farther) => read_or_empty(&farther.path(city_root)?)?,
                None => Vec::new(),
            };
            crate::document::revise(&path, |held, on_disk| {
                held_at(&path, on_disk, base)?;
                if path.exists() {
                    return Err(refused(doc, "this layer already has its own copy"));
                }
                held.replace(&start)?;
                Ok(PromptWritten {
                    bytes: start.len(),
                    removed: false,
                })
            })
        }
        (
            PromptStep::Inherit,
            PromptDocument::BuildingCity { .. } | PromptDocument::ModelNote { .. },
        ) => crate::document::revise(&path, |held, on_disk| {
            held_at(&path, on_disk, base)?;
            if !path.exists() {
                return Err(refused(doc, "this layer has no copy of its own to remove"));
            }
            held.remove()?;
            Ok(PromptWritten {
                bytes: 0,
                removed: true,
            })
        }),
        (PromptStep::Own | PromptStep::Inherit, PromptDocument::Shipped(_))
        | (
            PromptStep::Restore,
            PromptDocument::BuildingCity { .. } | PromptDocument::ModelNote { .. },
        ) => Err(refused(doc, "this step does not apply to this document")),
    }
}

fn held_at(path: &Path, on_disk: &[u8], base: B3Hash) -> Result<(), AxError> {
    if B3Hash::digest(on_disk) == base {
        Ok(())
    } else {
        Err(crate::document::moved(path))
    }
}

fn refused(doc: &PromptDocument, why: &str) -> AxError {
    let takes = match doc {
        PromptDocument::Shipped(_) => "write it back to this build's default (restore)",
        PromptDocument::BuildingCity { .. } | PromptDocument::ModelNote { .. } => {
            "start this layer's own copy (own) while it has none, or remove it (inherit) while \
             it has one"
        }
    };
    let at = doc.at().map_or_else(
        |_| "a prompt document".to_owned(),
        |at| at.as_str().to_owned(),
    );
    AxError::failure(
        AxCode::InvalidArgs,
        "change a prompt document",
        format!("{at}: {why}"),
    )
    .with_recovery(format!(
        "for this document you can {takes}; edit its words in the document view"
    ))
}

fn a_building(segment: &str) -> Option<Address> {
    Address::parse(segment)
        .ok()
        .filter(|addr| !addr.is_reserved())
}

fn note_of(building: Option<Address>, provider: &str, note: &[&str]) -> Option<PromptDocument> {
    let model = note.join("/");
    let model = model.strip_suffix(NOTE_EXT)?.strip_suffix('.')?;
    let doc = PromptDocument::ModelNote {
        building,
        provider: provider.to_owned(),
        model: model.to_owned(),
    };
    doc.at().ok()?;
    Some(doc)
}

fn note_spelling(building: Option<&Address>, provider: &str, model: &str) -> String {
    let note = format!("{RESERVED_PREFIX}/{MODELS_DIR}/{provider}/{model}.{NOTE_EXT}");
    match building {
        Some(building) => format!("{}/{note}", building.as_str()),
        None => note,
    }
}

fn names_in(dir: &Path) -> Result<Vec<String>, AxError> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(storage(dir, &err)),
    };
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| storage(dir, &err))?;
        if let Some(name) = entry.file_name().to_str() {
            names.push(name.to_owned());
        }
    }
    Ok(names)
}

fn read_or_empty(path: &Path) -> Result<Vec<u8>, AxError> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(bytes),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(err) => Err(storage(path, &err)),
    }
}

fn storage(path: &Path, err: &std::io::Error) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "read a prompt document",
        format!("{}: {err}", path.display()),
    )
    .with_recovery("make the file or its directory readable, then send the step again")
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
