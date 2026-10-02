// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The exit codes this binary speaks, one variant per code
//! (sprawling-SPEC.md 8-103; the table and the reading of `Unheard`
//! are modelled in `crates/sprawling/spec/Main/Exit.lean`).
//!
//! For an agent driving the binary the exit code is the result, so each
//! code asserts one fact and no two facts share a code: a city that is
//! not there (4) is not a city that refused (1), and a command line this
//! binary could not read (2) is neither.

use std::process::ExitCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Exit {
    /// 0: the work was done.
    Done,
    /// 1: the city or this machine refused, with an `AxError`.
    Refused,
    /// 2: this command line was not readable, including a `call` frame
    /// the wire cannot carry.
    Line,
    /// 3: `call` or `dispatch` sent its frame and nothing came back
    /// inside the quiet window, or the city spoke and the frame the verb
    /// waited for did not come (`Spoken::Quiet`, `Spoken::Unfinished`).
    Quiet,
    /// 4: nothing at the address answered as a city.
    NoCity,
}

impl From<Exit> for ExitCode {
    fn from(exit: Exit) -> Self {
        ExitCode::from(match exit {
            Exit::Done => 0,
            Exit::Refused => 1,
            Exit::Line => 2,
            Exit::Quiet => 3,
            Exit::NoCity => 4,
        })
    }
}
