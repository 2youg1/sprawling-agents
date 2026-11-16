// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::Address;

use super::BacklogId;
use super::ceiling::Ceiling;
use super::waiting::Exit;

#[derive(Debug, PartialEq, Eq)]
pub enum Started {
    Settled {
        exit: Exit,
        stdout: String,
        stderr: String,

        ceiling: Option<Ceiling>,
    },
    Backgrounded {
        id: BacklogId,
        what: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BacklogKind {
    Command,
    Run,
}

impl BacklogKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            BacklogKind::Command => "command",
            BacklogKind::Run => "run",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Standing {
    pub id: BacklogId,
    pub scope: Address,
    pub what: String,
    pub kind: BacklogKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finished {
    pub id: BacklogId,
    pub what: String,
    pub exit: Exit,
    pub stdout: String,
    pub stderr: String,

    pub ceiling: Option<Ceiling>,
}
