// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The `[resident]` table: which official harness a layer names as the
//! resident of the rooms below it, and which one a room runs.
//!
//! The five spellings are `agent_protocols::Harness`'s to answer, and
//! this crate sees only `kernel`, so a name is read as written and
//! judged where a dispatch is agreed (`crates/sprawling/Spec.lean` §8-4e, rule 10).
//! This module owns how one layer states it, the rule that a layer
//! names a model or a harness and never both, and the rule that a
//! session which opened on a model keeps it until `/new`
//! (`crates/city/spec/ConfigLayers.lean` §8-4, city D6).

use std::path::Path;

use kernel::{Address, AxError};
use serde::Deserialize;

use super::ladder::{Ladder, Layer};
use super::own_layer;
use super::refuse::{refuse, two_residents};

/// The session record's key, spelled once for every refusal that names
/// it.
pub(super) const MODEL_NAME_KEY: &str = "`[model] name`";

/// This table's key, spelled once for every refusal that names it.
pub(super) const HARNESS_KEY: &str = "`[resident] harness`";

/// What one layer's `[resident]` table states, as written.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ResidentSection {
    #[serde(default)]
    pub(super) harness: Option<String>,
}

/// A name as one key states it. An empty one states nothing and is a
/// slip of the pen, so it is refused rather than read as absent.
pub(super) fn stated_name(raw: Option<String>, key: &str) -> Result<Option<String>, AxError> {
    if raw.as_deref().is_some_and(|name| name.trim().is_empty()) {
        return Err(refuse(format!(
            "{key} is empty: leave the key out to state none"
        )));
    }
    Ok(raw)
}

/// One layer names one resident: the model a session recorded here or
/// a harness a person chose, never both.
pub(super) fn one_resident(model: Option<&str>, harness: Option<&str>) -> Result<(), AxError> {
    match (model, harness) {
        (Some(model), Some(harness)) => Err(two_residents(model, harness)),
        (Some(_), None) | (None, Some(_)) | (None, None) => Ok(()),
    }
}

/// The harness the next session at `addr` is handed to, and the rung
/// that named it.
///
/// `None` when no rung names one, and when the address's own file holds
/// a session's `[model] name`: that session opened on a model and keeps
/// it until `/new` forgets the record (`crates/city/spec/ConfigLayers.lean`
/// §8-4, city D6). The ladder is read first, so a rung that cannot be
/// read is reported before the record is consulted, as [`super::load`]
/// would report it.
///
/// # Errors
/// Refuses an address with no building, an unreadable file, and a file
/// that does not parse, exactly as [`super::load`] does.
pub fn settled_harness(
    city_root: &Path,
    addr: &Address,
) -> Result<Option<(String, Layer)>, AxError> {
    let ladder = Ladder::read(city_root, addr)?;
    if own_layer(city_root, addr)?.model().is_some() {
        return Ok(None);
    }
    Ok(ladder
        .tagged(|layer| layer.harness.clone())
        .resolve()
        .cloned())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;
    use crate::config_layers::refuse::{refuse, two_residents};
    use crate::config_layers::{ConfigLayer, path, write_session};

    fn state(city_root: &Path, room: &Address, layer: Layer, harness: &str) {
        let file = path(city_root, room, layer).unwrap();
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, format!("[resident]\nharness = \"{harness}\"\n")).unwrap();
    }

    #[test]
    fn one_layer_naming_a_model_and_a_harness_is_refused_naming_both() {
        assert_eq!(
            ConfigLayer::parse("[model]\nname = \"m-local\"\n\n[resident]\nharness = \"pi\"\n"),
            Err(two_residents("m-local", "pi"))
        );
    }

    #[test]
    fn an_empty_harness_states_nothing_and_is_refused() {
        assert_eq!(
            ConfigLayer::parse("[resident]\nharness = \"\"\n"),
            Err(refuse(
                "`[resident] harness` is empty: leave the key out to state none".to_owned()
            ))
        );
    }

    #[test]
    fn a_session_that_opened_on_a_model_keeps_it_until_new_and_then_the_nearest_harness_runs() {
        let dir = tempfile::tempdir().unwrap();
        let room = Address::parse("lab/room1").unwrap();
        state(dir.path(), &room, Layer::City, "codex");
        state(dir.path(), &room, Layer::Building, "pi");
        assert_eq!(
            settled_harness(dir.path(), &room).unwrap(),
            Some(("pi".to_owned(), Layer::Building)),
            "the nearest rung names the harness, and says which rung it is"
        );

        write_session(dir.path(), &room, "m-local", None).unwrap();
        assert_eq!(
            settled_harness(dir.path(), &room).unwrap(),
            None,
            "the session that opened on a model keeps it"
        );

        crate::forget_shape(dir.path(), &room).unwrap();
        assert_eq!(
            settled_harness(dir.path(), &room).unwrap(),
            Some(("pi".to_owned(), Layer::Building)),
            "/new hands the room to the harness"
        );
    }
}
