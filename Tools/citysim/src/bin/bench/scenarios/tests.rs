// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn small() -> Fixture {
    Fixture {
        fold_records: 40,
        fold_rounds: 1,
        tree_files: 3,
        tree_file_bytes: 32,
        placements: 1,
        forward_events: 4,
        pinned: "",
    }
}

const KEYS: [&str; 10] = [
    "perf",
    "load",
    "sub",
    "machine_class",
    "fixture",
    "samples",
    "floor_us",
    "p50_us",
    "p95_us",
    "p99_us",
];

fn keys(line: &str) -> Vec<&str> {
    line.split_whitespace()
        .filter_map(|field| field.split_once('=').map(|(key, _)| key))
        .collect()
}

#[test]
fn every_load_scenario_reruns_and_emits_the_stable_format() {
    let fixture = small();
    for round in 0..2 {
        let dir = tempfile::tempdir().unwrap();
        let taken = Taken {
            machine: super::super::reading::MachineClass::General,
            fixture: B3Hash::digest(b"small"),
        };
        let readings = all(dir.path(), &fixture, taken).unwrap();
        assert!(!readings.is_empty(), "round {round} produced no readings");
        for reading in &readings {
            let line = reading.line();
            let mut spelled: Vec<&str> = Vec::new();
            spelled.push("perf");
            spelled.extend(keys(&line));
            assert_eq!(spelled, KEYS, "round {round} spelled {line}");
            assert!(
                line.contains("machine_class=general"),
                "round {round} lost the machine class: {line}"
            );
        }
    }
}

#[test]
fn the_registered_fixture_writes_the_bytes_its_digest_pins() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        REGISTERED.digest(dir.path()).unwrap().to_string(),
        REGISTERED.pinned
    );
}
