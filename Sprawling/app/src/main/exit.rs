// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::process::ExitCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Exit {

    Done,

    Refused,

    Line,

    Quiet,

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
