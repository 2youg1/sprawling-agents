// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The prompt layers of one address, read off the disk when a page asks
//! (`crates/accounting/Spec.lean` §8-38, `crates/wire/Spec.lean` §8-98).
//!
//! Every judgement here is one the freeze already makes: which `City.md`
//! governs a building is `city::city_prompt_path`, a convention file is
//! read by `prompt_chain`, and a model's note is the building's when it
//! keeps one. The answer only lays those out in the order they are sent.

use std::path::Path;

use kernel::{Address, AxError, B3Hash};
use wire::{
    DefaultStanding, LayerDocument, LayerState, LayersAnswer, PrefixSlot, PromptKind, PromptLayer,
    ShippedDefault,
};

use super::prepared::unavailable_because;
use crate::prompt_chain::{self, Reading, addressed};

/// The prompt documents of the next run at `addr`, or `Unavailable`
/// with the city's reason for an address no building holds.
pub(super) fn layers_answer(city_root: &Path, addr: Address) -> wire::Answer {
    match layers(city_root, &addr) {
        Ok(documents) => wire::Answer::Layers(Box::new(LayersAnswer { addr, documents })),
        Err(stopped) => unavailable_because("Layers".to_owned(), &stopped),
    }
}

/// The four slots in send order, the farther layer first within each.
fn layers(city_root: &Path, addr: &Address) -> Result<Vec<LayerDocument>, AxError> {
    let building = city::Building::of(addr)?;
    let home = building.addr();
    let mut out = city_slot(city_root, home)?;
    for (layer, convention) in [PromptLayer::City, PromptLayer::Building]
        .into_iter()
        .zip(prompt_chain::building_conventions(city_root, home))
    {
        out.push(convention_document(
            city_root,
            home,
            &convention.path,
            (PrefixSlot::Building, layer),
        )?);
    }
    out.push(identity(city_root, addr, home)?);
    for convention in prompt_chain::room_conventions(city_root, addr, home)? {
        out.push(convention_document(
            city_root,
            home,
            &convention.path,
            (PrefixSlot::Resident, PromptLayer::Room),
        )?);
    }
    out.extend(model_notes(city_root, home)?);
    Ok(out)
}

/// The city's `City.md` and the building's own, of which the nearer one
/// that is there governs (city D31).
fn city_slot(city_root: &Path, home: &Address) -> Result<Vec<LayerDocument>, AxError> {
    let governing = city::city_prompt_path(city_root, home)?;
    let city = city::PromptDocument::Shipped(city::Shipped::City);
    let own = city::PromptDocument::BuildingCity {
        building: home.clone(),
    };
    let own_at = own.at()?;
    let own_path = own.path(city_root)?;
    let (city_state, city_version) = on_disk(&city.path(city_root)?);
    let city_state = if governing == own_path {
        LayerState::Replaced { by: own_at.clone() }
    } else {
        city_state
    };
    let (own_state, own_version) = on_disk(&own_path);
    Ok(vec![
        LayerDocument {
            at: city.at()?,
            version: city_version,
            slot: PrefixSlot::City,
            kind: PromptKind::City,
            layer: PromptLayer::City,
            state: city_state,
            shipped: Some(shipped(city_root, city::Shipped::City)?),
        },
        LayerDocument {
            at: own_at,
            version: own_version,
            slot: PrefixSlot::City,
            kind: PromptKind::City,
            layer: PromptLayer::Building,
            state: own_state,
            shipped: None,
        },
    ])
}

/// One convention file, read as the freeze reads it.
fn convention_document(
    city_root: &Path,
    home: &Address,
    path: &Path,
    (slot, layer): (PrefixSlot, PromptLayer),
) -> Result<LayerDocument, AxError> {
    let (state, version) = match prompt_chain::read_convention(city_root, home, path)? {
        Reading::Sent(bytes) => (
            LayerState::Sent {
                bytes: length(bytes.len()),
            },
            B3Hash::digest(&bytes),
        ),
        Reading::Absent => (LayerState::Absent, nothing()),
        Reading::Withheld(why) => (LayerState::Withheld { why }, nothing()),
    };
    Ok(LayerDocument {
        at: addressed(city_root, path)?,
        version,
        slot,
        kind: PromptKind::Conventions,
        layer,
        state,
        shipped: None,
    })
}

/// Who the resident at `addr` is: a room's own file, or for a City Hall
/// seat the Mayor's or the clerk's, which the city keeps and this build
/// carries a default for.
fn identity(city_root: &Path, addr: &Address, home: &Address) -> Result<LayerDocument, AxError> {
    let path = city::urbanite_path(city_root, addr);
    let at = addressed(city_root, &path)?;
    let (layer, shipped_doc) = match city::PromptDocument::of(&at) {
        Some(city::PromptDocument::Shipped(doc)) => (PromptLayer::City, Some(doc)),
        Some(
            city::PromptDocument::BuildingCity { .. } | city::PromptDocument::ModelNote { .. },
        )
        | None => {
            let layer = if addr == home {
                PromptLayer::Building
            } else {
                PromptLayer::Room
            };
            (layer, None)
        }
    };
    let (state, version) = on_disk(&path);
    Ok(LayerDocument {
        at,
        version,
        slot: PrefixSlot::Resident,
        kind: PromptKind::Identity,
        layer,
        state,
        shipped: shipped_doc.map(|doc| shipped(city_root, doc)).transpose()?,
    })
}

/// Every note the city and the building keep, one model at a time: the
/// city's note, replaced by the building's where the building keeps one,
/// and the building's place for that model, absent when it keeps none,
/// so a page can start one there without spelling the address itself.
fn model_notes(city_root: &Path, home: &Address) -> Result<Vec<LayerDocument>, AxError> {
    let city = city::model_notes(city_root, None)?;
    let own = city::model_notes(city_root, Some(home))?;
    let models: std::collections::BTreeSet<&(String, String)> = city.iter().chain(&own).collect();
    let mut out = Vec::new();
    for model in models {
        let own_at = note_at(Some(home), model)?;
        if city.contains(model) {
            let at = note_at(None, model)?;
            let (state, version) = on_disk(&city_root.join(at.as_str()));
            let state = if own.contains(model) {
                LayerState::Replaced { by: own_at.clone() }
            } else {
                state
            };
            out.push(note(at, version, PromptLayer::City, state, model));
        }
        let (state, version) = on_disk(&city_root.join(own_at.as_str()));
        out.push(note(own_at, version, PromptLayer::Building, state, model));
    }
    Ok(out)
}

fn note(
    at: Address,
    version: B3Hash,
    layer: PromptLayer,
    state: LayerState,
    (provider, model): &(String, String),
) -> LayerDocument {
    LayerDocument {
        at,
        version,
        slot: PrefixSlot::Run,
        kind: PromptKind::ModelNote {
            provider: provider.clone(),
            model: model.clone(),
        },
        layer,
        state,
        shipped: None,
    }
}

/// Where a note the city listed is kept.
///
/// # Errors
/// `E_INVALID_ARGS` for a name the address grammar refuses, which a note
/// `city::model_notes` listed cannot carry.
fn note_at(building: Option<&Address>, (provider, model): &(String, String)) -> Result<Address, AxError> {
    city::model_note_at(building, provider, model).ok_or_else(|| {
        AxError::failure(
            kernel::AxCode::InvalidArgs,
            "list a model's note",
            format!("{provider}/{model}"),
        )
        .with_recovery("rename the note to letters, digits, dashes and dots")
    })
}

/// Whether a document the city keeps in a reserved subtree, or at its
/// root, is there for the freeze to read, and the version a step on it
/// starts from. The freeze reads these files as they stand, so anything
/// there that is not a plain file, or that will not read, is withheld
/// with the disk's own words.
fn on_disk(path: &Path) -> (LayerState, B3Hash) {
    match std::fs::read(path) {
        Ok(bytes) => (
            LayerState::Sent {
                bytes: length(bytes.len()),
            },
            B3Hash::digest(&bytes),
        ),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => (LayerState::Absent, nothing()),
        Err(err) => (
            LayerState::Withheld {
                why: format!("{}: {err}", path.display()),
            },
            nothing(),
        ),
    }
}

/// The version of no bytes: what a document that is not there is, and
/// the `base` a step that creates it starts from.
fn nothing() -> B3Hash {
    B3Hash::digest(b"")
}

/// This build's default for `doc`, and the city's copy measured against
/// it.
///
/// # Errors
/// `E_STORAGE_FATAL` naming the path for a copy that is there and will
/// not read.
fn shipped(city_root: &Path, doc: city::Shipped) -> Result<ShippedDefault, AxError> {
    let path = doc.path(city_root);
    let copy = match std::fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(err) => {
            return Err(AxError::failure(
                kernel::AxCode::StorageFatal,
                "read a document whose default this build carries",
                format!("{}: {err}", path.display()),
            )
            .with_recovery("make the file readable, then ask again"));
        }
    };
    Ok(ShippedDefault {
        version: B3Hash::digest(doc.text().as_bytes()),
        standing: match city::standing(doc, copy.as_deref()) {
            city::CopyStanding::Absent => DefaultStanding::Absent,
            city::CopyStanding::Current => DefaultStanding::Current,
            city::CopyStanding::Earlier => DefaultStanding::Earlier,
            city::CopyStanding::Edited => DefaultStanding::Edited,
        },
    })
}

fn length(bytes: usize) -> u64 {
    u64::try_from(bytes).unwrap_or(u64::MAX)
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
