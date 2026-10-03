// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::Tool;

use super::tests::desk;
use super::*;

/// The plan entry's catalog bytes are pinned. The catalog is what every
/// turn pays for, so a verb that grows it is a verb charged to every run
/// in the city whether or not it is ever called.
#[test]
fn the_plan_entry_costs_no_more_catalog_bytes_than_its_pin() {
    let tool = ClaimTool::new(desk()).unwrap();
    let meta = tool.meta();
    let bytes = meta.disclosure.len()
        + serde_json::to_string(meta.params.as_map())
            .expect("the schema serialises")
            .len();
    // The four-action tool measured 548 B on this same reading. Two
    // more verbs and two more arguments fit under it because the
    // locator grammar left the schema for the refusal that needs it:
    // a description repeating what a refusal already says is paid
    // for every turn and read once. The seventh verb, `add`, is the only
    // door into an empty plan (roadmap F2) and costs 33 B: it reuses
    // `parts` instead of carrying a schema field of its own.
    assert!(meta.disclosure.contains("Must this be expanded?"));
    assert!(
        bytes <= 581,
        "the plan entry costs {bytes} B, and its pin is 581"
    );
}
