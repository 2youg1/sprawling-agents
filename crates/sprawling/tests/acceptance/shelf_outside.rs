// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Every skill this repository ships, on a shelf the city mounts from
//! outside itself and admitted by a building's reading room, is pinned
//! when a run starts and read by name as the bytes the pin names
//! (`crates/runtime/Spec.lean` §8-29-6; `docs/getting-started.md`, "Tools, skills
//! and MCP").
//!
//! A shipped skill's pinned bytes are the staged ones, the licence list
//! riding inside (`city` D37): the library shelf is nearer than the
//! mounted one (`Library.lean` §8-8), so for a shipped name the city's
//! staged copy is the holding the pin names.
//!
//! The shelf is the `skills/` directory itself, named in the city's own
//! configuration the way the guide tells a person to name one.

use std::path::Path;

use kernel::B3Hash;
use serde_json::json;

use crate::city::{self, History};
use crate::script::{self, Step};
use crate::skills::{Shipped, shipped, skills_dir};

/// What the run was dispatched with and what reached the model.
#[derive(Debug, PartialEq)]
struct Seen {
    /// The skills `run_started` pinned, with their hashes.
    pinned: Vec<(String, B3Hash)>,
    /// The skills whose `SKILL.md` came back from a `read` by name.
    read: Vec<String>,
}

#[test]
fn every_shipped_skill_on_a_shelf_outside_the_city_is_read_by_name_and_pinned() {
    let shipped = shipped();
    let dir = tempfile::tempdir().unwrap();
    let steps = shipped
        .iter()
        .map(|skill| Step {
            tool: "read",
            args: json!({ "file_path": skill.name }),
        })
        .collect();
    let (factory, _, heard) = script::scripted(steps);
    let (mut worker, ledger) = city::city_with_a_model(dir.path(), factory);
    city::raise(&mut worker, "lab", "minimal");
    mount(dir.path());
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
        },
        Seen {
            pinned: shipped.iter().map(as_shelved).collect(),
            read: shipped.iter().map(|skill| skill.name.clone()).collect(),
        },
        "the dispatch answered {dispatched:?}"
    );
}

/// Names `skills/` as the city's one shelf from outside, in the city's
/// own configuration, as an absolute path.
fn mount(city_root: &std::path::Path) {
    let config = kernel::layout::CityLayout::new(city_root).city_config();
    let shelf = std::fs::canonicalize(skills_dir()).unwrap();
    let mut text = std::fs::read_to_string(&config).unwrap_or_default();
    assert!(
        !text.contains("[skills]"),
        "the founded city already names shelves: {text}"
    );
    text.push_str(&format!("\n[skills]\nshelves = ['{}']\n", shelf.display()));
    std::fs::create_dir_all(config.parent().unwrap()).unwrap();
    std::fs::write(&config, text).unwrap();
}

/// The canonical manifest digest a run pins for a shipped skill: the
/// staged bytes, the licence list riding inside (`city` D37). The
/// library is the nearer shelf (`Library.lean` §8-8), so its staged
/// copy of a shipped name is the holding a run pins, not the raw
/// directory the mounted shelf holds.
fn as_shelved(skill: &Shipped) -> (String, B3Hash) {
    let staged = tempfile::tempdir().unwrap();
    copy_dir(&skill.dir, staged.path());
    std::fs::copy(
        skills_dir().join("LICENSES.md"),
        staged.path().join("LICENSES.md"),
    )
    .unwrap();
    (
        skill.name.clone(),
        ::city::skill_digest(staged.path()).unwrap(),
    )
}

/// Copies one directory tree without changing a byte.
fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}
