// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How a table is made and configured: the window its callers block for
//! before a command is handed to the background
//! (`crates/runtime/spec/Backlog.lean` §8-28), where a command's output
//! goes while it is inside that window (the same part, §8-28-3), and what
//! shares every run it starts asks for
//! (`crates/runtime/spec/Tools/Exec.lean` D29). The table itself, what it
//! carries and how it ends, is the module above.

use super::{Backlog, PollBudget, Scratch, Shares, Sink};

impl Backlog {
    #[must_use]
    pub fn new() -> Backlog {
        Backlog::default()
    }

    /// A table whose callers block for this window before a command is
    /// handed to the background.
    #[must_use]
    pub fn with_window(window: PollBudget) -> Backlog {
        Backlog {
            table: std::sync::Arc::default(),
            scratch: Scratch::open(),
            window,
            sink: None,
            shares: Shares::Unset,
        }
    }

    /// The same table, handing what a command writes to `sink` while
    /// the command is still inside its window (`crates/runtime/spec/Backlog.lean` §8-28-3).
    #[must_use]
    pub fn with_sink(self, sink: Sink) -> Backlog {
        Backlog {
            sink: Some(sink),
            ..self
        }
    }

    /// The same table, asking these shares for every run whose command
    /// it starts (`crates/runtime/spec/Tools/Exec.lean` D29).
    #[must_use]
    pub fn with_shares(self, shares: Shares) -> Backlog {
        Backlog { shares, ..self }
    }

    /// The shares this table asks for each run.
    #[must_use]
    pub fn shares(&self) -> Shares {
        self.shares
    }
}
