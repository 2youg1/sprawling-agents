// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![no_main]

use libfuzzer_sys::fuzz_target;

// The text of one CONFIG.toml, as a person typed it.
//
// Property under fuzz: reading a layer never panics; an accepted layer
// reads the same way twice; and the two rules the layer alone can
// enforce hold on everything it accepts - no sandbox mount reaches into
// the city's own subtree, and no two MCP servers share a label.
fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(layer) = city::ConfigLayer::parse(text) else {
        return; // refusing a malformed file is the ordinary outcome
    };
    assert_eq!(
        city::ConfigLayer::parse(text).ok().as_ref(),
        Some(&layer),
        "one file text states one configuration"
    );

    if let Some(sandbox) = layer.sandbox() {
        for mount in &sandbox.mounts {
            assert!(
                !mount.is_reserved(),
                "a mount into the reserved subtree would hand an agent its own configuration"
            );
        }
    }
    if let Some(servers) = layer.mcp() {
        for (at, server) in servers.iter().enumerate() {
            let before = servers.get(..at).unwrap_or_default();
            assert!(
                !before.iter().any(|held| held.label == server.label),
                "two servers under one label put one tool name in front of two processes"
            );
        }
    }
});
