// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The prompt documents a page may write at the city root or inside a
//! reserved subtree, and the three whole-document steps a person takes
//! on them (`crates/city/Spec.lean` §8-44, city D33).
//!
//! A reserved subtree is out of every write domain so that no resident
//! rewrites what it is told; the city's own doors still let a person
//! write a few files there, and this module is the one list of them.

use std::path::{Path, PathBuf};

use kernel::{Address, AxCode, AxError, B3Hash, RESERVED_PREFIX};

use crate::defaults::Shipped;
use crate::spine_files::CITY_FILE;
use crate::spine_files::hall::{CLERK_FILE, MAYOR_FILE};

/// The directory under a reserved prefix that holds one directory per
/// provider, each holding one note per model.
const MODELS_DIR: &str = "models";

/// The extension a model's note is written under.
const NOTE_EXT: &str = "md";

/// One prompt document a page may write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptDocument {
    /// `City.md` at the city root, `MAYOR.md` or `CLERK.md`: the three
    /// whose default text this build carries.
    Shipped(Shipped),
    /// A building's own `City.md`, in its reserved subtree (city D31).
    BuildingCity { building: Address },
    /// The note a person keeps for one model: the city's, or a
    /// building's, which replaces the city's for that model.
    ModelNote {
        building: Option<Address>,
        provider: String,
        model: String,
    },
}

/// A whole-document step on one prompt document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptStep {
    /// This layer starts its own copy from the one that governs now.
    Own,
    /// This layer's copy goes, and the farther one governs again.
    Inherit,
    /// This build's default text is written back.
    Restore,
}

/// What one step left: how long the document now is, and whether it is
/// gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PromptWritten {
    pub bytes: usize,
    pub removed: bool,
}

impl PromptDocument {
    /// The prompt document `at` names, or `None` when it is none of
    /// them.
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
            [RESERVED_PREFIX, MODELS_DIR, provider, note] => note_of(None, provider, note),
            [building, RESERVED_PREFIX, MODELS_DIR, provider, note] => {
                note_of(Some(a_building(building)?), provider, note)
            }
            _ => None,
        }
    }

    /// The address this document is kept at.
    ///
    /// # Errors
    /// A provider or model the address grammar refuses, which a document
    /// [`PromptDocument::of`] read cannot hold.
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

    /// Where this document is in the city rooted at `city_root`.
    ///
    /// # Errors
    /// As [`PromptDocument::at`].
    pub fn path(&self, city_root: &Path) -> Result<PathBuf, AxError> {
        Ok(city_root.join(self.at()?.as_str()))
    }

    /// Refuses text this document may not hold: the Mayor's identity
    /// area must read, by the reader the settings page's door holds, so
    /// a word-by-word save cannot leave the Mayor nameless.
    ///
    /// # Errors
    /// `E_CONFIG_INVALID` naming the line that does not read, and
    /// `E_INVALID_ARGS` for Mayor text that is not UTF-8.
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

    /// The document one layer out, which governs where this one is
    /// absent: the city's `City.md` for a building's, the city's note for
    /// a building's note. `None` for a document no farther layer stands
    /// behind.
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

/// Where the note for `model` on the endpoint named `provider` is kept:
/// in `building`'s reserved subtree, or the city's when `building` is
/// `None`. Spelled as an address before it is ever a path, so an id
/// with a `..` segment names no note rather than a file outside the
/// reserved subtree; `None` for a provider or model the grammar refuses.
#[must_use]
pub fn model_note_at(building: Option<&Address>, provider: &str, model: &str) -> Option<Address> {
    Address::parse(&note_spelling(building, provider, model)).ok()
}

/// Every note one layer keeps, as `(provider, model)`, in byte order: the
/// city's when `building` is `None`.
///
/// # Errors
/// `E_STORAGE_FATAL` naming the directory when it is there and cannot
/// be listed.
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
        for file in names_in(&root.join(&provider))? {
            if let Some(PromptDocument::ModelNote {
                provider, model, ..
            }) = note_of(None, &provider, &file)
            {
                notes.push((provider, model));
            }
        }
    }
    notes.sort();
    Ok(notes)
}

/// Takes `step` on `doc`, only while the file still holds the version
/// `base` names (a file that is not there reads as no bytes).
///
/// `Own` starts a copy from the farther layer's (empty when there is
/// none) and refuses a file that is already there; `Inherit` removes the
/// file; `Restore` writes this build's default back through
/// [`crate::defaults::restore`], which keeps an identity area.
///
/// # Errors
/// `E_INVALID_ARGS` for a step this document does not take, and for
/// `Own` on a file already there or `Inherit` on one that is not;
/// `E_VERSION_CONFLICT` when the file is no longer `base`;
/// `E_STORAGE_FATAL` naming the path when a file cannot be read,
/// written or removed.
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

/// Refuses a file that moved since the page read the version `base`
/// names.
fn held_at(path: &Path, on_disk: &[u8], base: B3Hash) -> Result<(), AxError> {
    if B3Hash::digest(on_disk) == base {
        Ok(())
    } else {
        Err(crate::document::moved(path))
    }
}

/// The refusal for a step `doc` does not take, with the steps it does.
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

/// A segment that names a building: one segment, outside every reserved
/// subtree.
fn a_building(segment: &str) -> Option<Address> {
    Address::parse(segment)
        .ok()
        .filter(|addr| !addr.is_reserved())
}

/// The note `file` names under `provider`, when `file` carries the
/// note extension and both parts read back to the same address.
fn note_of(building: Option<Address>, provider: &str, file: &str) -> Option<PromptDocument> {
    let model = file.strip_suffix(NOTE_EXT)?.strip_suffix('.')?;
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

/// The names in `dir`, or none when it is not there.
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

/// The file's bytes, or none when it is not there.
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
