// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::Tool;

use super::tests::desk;
use super::*;

#[test]
fn the_plan_entry_costs_no_more_catalog_bytes_than_its_pin() {
    let tool = ClaimTool::new(desk()).unwrap();
    let meta = tool.meta();
    let bytes = meta.disclosure.len()
        + serde_json::to_string(meta.params.as_map())
            .expect("the schema serialises")
            .len();

    assert!(meta.disclosure.contains("Must this be expanded?"));
    assert!(
        bytes <= 581,
        "the plan entry costs {bytes} B, and its pin is 581"
    );
}
