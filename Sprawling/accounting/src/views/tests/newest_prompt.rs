// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::{Place, view_record};
use crate::views::Views;
use crate::views::holding::RunPrompt;
use kernel::{Address, EventKind, RunId, Seq};

/// A building's capabilities come from the run it started last, even
/// when a run started earlier in another room freezes its prompt after
/// it (`crates/accounting/spec/Views/Usage.lean`, the capabilities
/// question).
#[test]
fn a_building_keeps_the_prompt_of_the_run_it_started_last() {
    let dir = tempfile::tempdir().unwrap();
    let mut views = Views::new(dir.path());
    let (east, west) = (
        Address::parse("lab/east").unwrap(),
        Address::parse("lab/west").unwrap(),
    );
    let (older, newer) = (RunId::from_bytes([1u8; 16]), RunId::from_bytes([2u8; 16]));
    let lines = [
        (1, older, EventKind::RunStarted, &east),
        (2, newer, EventKind::RunStarted, &west),
        (3, newer, EventKind::PromptAssembled, &west),
        (4, older, EventKind::PromptAssembled, &east),
    ];
    for (seq, run, kind, addr) in lines {
        views
            .apply(&view_record(
                Place { seq, run },
                kind,
                addr,
                serde_json::Map::new(),
            ))
            .unwrap();
    }

    assert_eq!(
        views
            .building_prompts
            .get(&Address::parse("lab").unwrap())
            .copied(),
        Some(RunPrompt {
            started: Seq::new(2),
            prompt: Seq::new(3),
        })
    );
}
