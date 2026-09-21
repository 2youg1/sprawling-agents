// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![no_main]

use libfuzzer_sys::fuzz_target;

// One line an MCP child process wrote to its stdout.
//
// Property under fuzz: reading an answer line never panics; a line
// carrying an `error` object is never read as a result, because a
// server's refusal handed back as a value is the one failure this city
// cannot see; and a `tools/list` result that is admitted names every
// tool under this server's prefix, which is what routes a later call
// back to the process that offered it.
fuzz_target!(|data: &[u8]| {
    let Ok(line) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(result) = protocol::Rpc::read(line) else {
        return; // refusing an unreadable line is the ordinary outcome
    };
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(line) {
        assert!(
            parsed.get("error").is_none(),
            "an answer carrying an error object is a refusal, never a result"
        );
    }

    let server = kernel::ServerLabel::parse("probe").expect("a fixed label parses");
    let Ok(listed) = protocol::tools_from(&server, &result) else {
        return;
    };
    let prefix = format!("{}_", server.as_str());
    for tool in &listed {
        assert!(
            tool.meta.name.as_str().starts_with(&prefix),
            "a tool reached through this server is named under it"
        );
    }
});
