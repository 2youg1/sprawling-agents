// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::{Address, AxError, B3Hash};
use wire::{
    DefaultStanding, LayerDocument, LayerState, LayersAnswer, PrefixSlot, PromptKind, PromptLayer,
    ShippedDefault,
};

use super::holding::Views;
use super::prepared::{Prepared, unavailable_because};
use crate::prompt_chain::{self, Reading, addressed};

impl Views {
    pub(in crate::views) fn layers_ask(&self, addr: &Address) -> Prepared {
        Prepared::Layers {
            city_root: self.city_root.clone(),
            addr: addr.clone(),
        }
    }
}

pub(super) fn layers_answer(city_root: &Path, addr: Address) -> wire::Answer {
    match layers(city_root, &addr) {
        Ok(documents) => wire::Answer::Layers(Box::new(LayersAnswer { addr, documents })),
        Err(stopped) => unavailable_because("Layers".to_owned(), &stopped),
    }
}

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
            out.push(note(at, PromptLayer::City, (state, version), model));
        }
        let found = on_disk(&city_root.join(own_at.as_str()));
        out.push(note(own_at, PromptLayer::Building, found, model));
    }
    Ok(out)
}

fn note(
    at: Address,
    layer: PromptLayer,
    (state, version): (LayerState, B3Hash),
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

fn note_at(
    building: Option<&Address>,
    (provider, model): &(String, String),
) -> Result<Address, AxError> {
    city::model_note_at(building, provider, model).ok_or_else(|| {
        AxError::failure(
            kernel::AxCode::InvalidArgs,
            "list a model's note",
            format!("{provider}/{model}"),
        )
        .with_recovery("rename the note to letters, digits, dashes and dots")
    })
}

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

fn nothing() -> B3Hash {
    B3Hash::digest(b"")
}

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
