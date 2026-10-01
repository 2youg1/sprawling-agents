// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `sprawling desktop [scope]`: this machine's desktop, served as an MCP
//! server over this process's own pipes (sprawling-SPEC.md 8-4d).
//!
//! A city starts it as a child for a building whose rules ask for the
//! desktop, naming that building's `DESKTOP.toml`; a person can start
//! it the same way for any other MCP client. No scope file closes every
//! window, which is what the server does with a file it cannot read too.

use std::path::Path;
use std::process::ExitCode;

/// Serves until the caller closes this process's input.
pub(super) fn verb(scope: Option<&str>) -> ExitCode {
    match ::desktop::serve_stdio(scope.map(Path::new)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            // stdout belongs to the protocol, so the one line a caller's
            // diagnostics can read goes to stderr.
            eprintln!("sprawling desktop: {err}");
            ExitCode::FAILURE
        }
    }
}
