// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where this server is started from: one argument, one environment
//! variable, one pair of pipes.
//!
//! A city starts it as a child process from a building's `CONFIG.toml`,
//! so the scope file has to be nameable on the command line. The
//! environment variable is for the same server started by hand. Neither
//! is required, and giving neither closes the scope — which is what a
//! server with no scope file has to do.

use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let named = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("SPRAWLING_DESKTOP_SCOPE").ok());
    match sprawling_desktop::serve_stdio(named.as_deref().map(Path::new)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            // The caller's own diagnostics are the only thing stderr
            // carries; stdout belongs to the protocol.
            eprintln!("sprawling-desktop: {err}");
            ExitCode::FAILURE
        }
    }
}
