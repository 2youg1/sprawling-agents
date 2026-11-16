// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::{Address, AxError};
use serde::Deserialize;

use super::ladder::{Ladder, Layer};
use super::own_layer;
use super::refuse::{refuse, two_residents};

pub(super) const MODEL_NAME_KEY: &str = "`[model] name`";

pub(super) const HARNESS_KEY: &str = "`[resident] harness`";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ResidentSection {
    #[serde(default)]
    pub(super) harness: Option<String>,
}

pub(super) fn stated_name(raw: Option<String>, key: &str) -> Result<Option<String>, AxError> {
    if raw.as_deref().is_some_and(|name| name.trim().is_empty()) {
        return Err(refuse(format!(
            "{key} is empty: leave the key out to state none"
        )));
    }
    Ok(raw)
}

pub(super) fn one_resident(model: Option<&str>, harness: Option<&str>) -> Result<(), AxError> {
    match (model, harness) {
        (Some(model), Some(harness)) => Err(two_residents(model, harness)),
        (Some(_), None) | (None, Some(_)) | (None, None) => Ok(()),
    }
}

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

        write_session(
            dir.path(),
            &room,
            &crate::SessionShape::new("m-local", None, None),
        )
        .unwrap();
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
