// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the table tells a caller: how a start came out, what is still
//! running, and what has stopped.
//!
//! Four value types and no behaviour. They sit apart from the table
//! because they are what everything outside this module reads, while
//! the table is how it is kept (runtime-SPEC 8-28-1).

use kernel::Address;

use super::BacklogId;
use super::waiting::Exit;

/// What the short window came to. Exhaustive rather than an optional
/// handle: "it finished" and "it is still going" are two different
/// things for the caller to say to a model, and a `None` would leave
/// which one it was to be inferred.
#[derive(Debug, PartialEq, Eq)]
pub enum Started {
    Settled {
        exit: Exit,
        stdout: String,
        stderr: String,
    },
    Backgrounded {
        id: BacklogId,
        what: String,
    },
}

/// Which of the two kinds of member a standing entry is.
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

/// A member that is still running.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Standing {
    pub id: BacklogId,
    pub scope: Address,
    pub what: String,
    pub kind: BacklogKind,
}

/// A member that has stopped, collected once and then forgotten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finished {
    pub id: BacklogId,
    pub what: String,
    pub exit: Exit,
    pub stdout: String,
    pub stderr: String,
}
