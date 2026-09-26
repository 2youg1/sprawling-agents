// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A pursuit's verdict crosses the wire as the kernel's kind, not as a
//! sentence: the client takes the words for it from `lang.json`, so a
//! page in any language reads the same fact.

#![allow(clippy::unwrap_used, reason = "test code")]

use channels::PursuitLine;
use serde_json::json;

#[test]
fn a_pursuit_verdict_travels_as_its_kind_and_not_as_a_sentence() {
    let wire = json!([
        { "addr": "acme", "goal": "ship it", "state": "running",
          "verdict": { "kind": "work", "next": "2.3" } },
        { "addr": "acme", "goal": "ship it", "state": "running",
          "verdict": { "kind": "waiting", "in_flight": 2 } },
        { "addr": "acme", "goal": "ship it", "state": "paused",
          "verdict": { "kind": "paused" } },
        { "addr": "acme", "goal": "ship it", "state": "running",
          "verdict": { "kind": "finished" } },
    ]);
    let read = serde_json::from_value::<Vec<PursuitLine>>(wire.clone())
        .map(|lines| serde_json::to_value(lines).unwrap());
    assert_eq!(read.ok(), Some(wire));
}
