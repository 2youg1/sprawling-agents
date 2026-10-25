// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The conventions of the rooms between a building's root and a run's
//! room: carried in the resident slot, so the building slot stays one
//! string for every room of the building.

use std::sync::{Arc, Mutex, PoisonError};

use super::super::*;
use crate::worker::fixture::*;
use crate::worker::*;

/// One recorded segment: its slot, its hash, and the addresses of the
/// documents it names.
type Segment = (String, String, Vec<String>);

fn watch(worker: &mut RunWorker) -> Arc<Mutex<Vec<Vec<Segment>>>> {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    worker.observe(Box::new(move |record: &EventRecord| {
        if record.kind() != EventKind::PromptAssembled {
            return;
        }
        let segments = record.data().as_map()["segments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                (
                    row["slot"].as_str().unwrap().to_owned(),
                    row["hash"].as_str().unwrap().to_owned(),
                    row["sources"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|source| source["addr"].as_str().unwrap().to_owned())
                        .collect(),
                )
            })
            .collect();
        sink.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(segments);
    }));
    seen
}

fn dispatch(worker: &mut RunWorker, room: &str) {
    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse(room).unwrap(),
            task: "measure the thing".to_owned(),
            goal: "a number with a unit, then stop".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, room.as_bytes()),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();
}

fn slot<'a>(segments: &'a [Segment], name: &str) -> &'a Segment {
    segments.iter().find(|(slot, _, _)| slot == name).unwrap()
}

/// A package two levels down works under its parent's conventions and
/// its own, read after the building's; a sibling room is told neither,
/// and both rooms carry the same building slot byte for byte.
#[test]
fn a_rooms_conventions_ride_in_the_resident_slot_after_the_buildings() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![completion("done", None), completion("done", None)],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let seen = watch(&mut worker);
    worker
        .handle(wire::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: wire::TemplateName::parse("minimal").unwrap(),
            rules: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();
    let lab = dir.path().join("lab");
    std::fs::create_dir_all(lab.join("pkg").join("probe")).unwrap();
    std::fs::write(lab.join("AGENTS.md"), "Measurements are in millivolts.\n").unwrap();
    std::fs::write(
        lab.join("pkg").join("AGENTS.md"),
        "The package logs in kelvin.\n",
    )
    .unwrap();
    std::fs::write(
        lab.join("pkg").join("probe").join("AGENTS.md"),
        "The probe samples at one hertz.\n",
    )
    .unwrap();

    dispatch(&mut worker, "lab/pkg/probe");
    dispatch(&mut worker, "lab/room2");

    let asked = provider.bodies().join("\n");
    let building = asked.find("Measurements are in millivolts.").unwrap();
    let package = asked.find("The package logs in kelvin.").unwrap();
    let probe = asked.find("The probe samples at one hertz.").unwrap();
    assert!(building < package && package < probe, "{asked}");
    let seen = seen.lock().unwrap_or_else(PoisonError::into_inner);
    assert_eq!(seen.len(), 2);
    assert_eq!(
        slot(&seen[0], "resident").2,
        vec![
            "lab/pkg/AGENTS.md".to_owned(),
            "lab/pkg/probe/AGENTS.md".to_owned()
        ]
    );
    assert_eq!(slot(&seen[1], "resident").2, Vec::<String>::new());
    assert_eq!(slot(&seen[0], "building"), slot(&seen[1], "building"));
}
