// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

#[test]
fn a_container_declaration_reaches_the_frozen_sandbox_whole() {
    let image = format!("sha256:{}", "a".repeat(64));
    let parsed = ConfigLayer::parse(&format!(
        "[sandbox]\nshell = false\n[sandbox.container]\nimage = '{image}'\nuser = 1000\ncpu_millis = 1250\nmemory_bytes = 67108864\npids = 64\n"
    ));
    assert!(
        parsed.is_ok(),
        "an explicit container must reach the frozen exec configuration: {parsed:?}"
    );
    let sandbox = serde_json::to_value(parsed.unwrap().sandbox().unwrap()).unwrap();
    assert_eq!(sandbox["container"]["image"], serde_json::json!(image));
}
