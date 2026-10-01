// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Every skill this repository ships, installed into the city library
//! and admitted by a building's reading room, is pinned when a run
//! starts and read by name (accounting-SPEC.md section 2, the ninth
//! assertion).
//!
//! The set of skills is the `skills/` directory itself. They go in
//! through the city library, the path that lands every file of a
//! package inside the city, so a file a package carries is read here
//! too (accounting-SPEC.md section 12, decision 23); `shelf_outside.rs`
//! mounts the same directory as a shelf outside the city.

use std::path::{Path, PathBuf};

use kernel::B3Hash;
use serde_json::json;

use crate::city::{self, History};
use crate::script::{self, Step};

/// The file inside a package the run reads by `<name>/<relative path>`.
const CARRIED: &str = "how/references/critique-rubric.md";

/// What the run was dispatched with and what reached the model.
#[derive(Debug, PartialEq)]
struct Seen {
    /// The skills `run_started` pinned, with their hashes.
    pinned: Vec<(String, B3Hash)>,
    /// The skills whose `SKILL.md` came back from a `read` by name.
    read: Vec<String>,
    /// Whether the first line of the carried file came back.
    packaged: bool,
}

#[test]
fn every_shipped_skill_a_building_admits_is_read_by_name_and_pinned() {
    let shipped = shipped();
    let dir = tempfile::tempdir().unwrap();
    let (factory, _, heard) = script::scripted(steps(&shipped));
    let (mut worker, ledger) = city::city_with_a_model(dir.path(), factory);
    city::raise(&mut worker, "lab", "minimal");
    let installed = install(dir.path(), &shipped);
    let admitted: Vec<String> = shipped
        .iter()
        .map(|skill| format!("{:?}", skill.name))
        .collect();
    city::rules(
        dir.path(),
        "lab",
        &format!(
            "confidential = false\nwrite = \"everything\"\nreading_room = [{}]\n",
            admitted.join(", ")
        ),
    );
    let dispatched = city::dispatch(&mut worker, "lab/lead");

    let heard: Vec<String> = heard.lock().unwrap().values().cloned().collect();
    let told = |line: &str| heard.iter().any(|result| result.contains(line));
    let pinned = History::read(&ledger)
        .started()
        .first()
        .map(|started| {
            started
                .record
                .skills
                .iter()
                .map(|pin| (pin.name.clone(), pin.hash))
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(
        Seen {
            pinned,
            read: shipped
                .iter()
                .filter(|skill| told(&skill.name_line))
                .map(|skill| skill.name.clone())
                .collect(),
            packaged: told(&first_line(&skills_dir().join(CARRIED))),
        },
        Seen {
            pinned: installed,
            read: shipped.iter().map(|skill| skill.name.clone()).collect(),
            packaged: true,
        },
        "the dispatch answered {dispatched:?}"
    );
}

/// One package under `skills/`.
pub(crate) struct Shipped {
    pub(crate) name: String,
    pub(crate) dir: PathBuf,
    /// The `name:` line of its `SKILL.md`, which a read by name hands
    /// back whatever else the result is wrapped in.
    pub(crate) name_line: String,
}

pub(crate) fn skills_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../skills")
}

/// Every directory under `skills/` that holds a `SKILL.md`, by name.
pub(crate) fn shipped() -> Vec<Shipped> {
    let mut shipped: Vec<Shipped> = std::fs::read_dir(skills_dir())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|dir| dir.join("SKILL.md").is_file())
        .map(|dir| Shipped {
            name: dir.file_name().unwrap().to_string_lossy().into_owned(),
            name_line: std::fs::read_to_string(dir.join("SKILL.md"))
                .unwrap()
                .lines()
                .find(|line| line.starts_with("name:"))
                .unwrap()
                .trim_end()
                .to_owned(),
            dir,
        })
        .collect();
    shipped.sort_by(|a, b| a.name.cmp(&b.name));
    shipped
}

/// The calls the script makes: every skill read by its name, in the
/// order the reading room admits them, then one file a package carries.
fn steps(shipped: &[Shipped]) -> Vec<Step> {
    shipped
        .iter()
        .map(|skill| skill.name.as_str())
        .chain([CARRIED])
        .map(|path| Step {
            tool: "read",
            args: json!({ "path": path }),
        })
        .collect()
}

/// Installs every package into the city library, and answers the
/// `SKILL.md` hash each install reported for what the scan reads back.
/// The whole package's hash is the store's key and answers another
/// question (city-SPEC.md 8-28).
fn install(city_root: &Path, shipped: &[Shipped]) -> Vec<(String, B3Hash)> {
    let mut cas = storage::Cas::open(&kernel::layout::CityLayout::new(city_root).cas()).unwrap();
    let mut register = |bytes: &[u8]| cas.put(bytes).map_err(storage::StorageError::into_ax);
    let slot = ::city::Slot::library("shipped").unwrap();
    shipped
        .iter()
        .map(|skill| {
            let installed =
                ::city::install_skill(city_root, &slot, &skill.dir, &mut register).unwrap();
            (skill.name.clone(), installed.holding.hash)
        })
        .collect()
}

fn first_line(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_owned()
}
