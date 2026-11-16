// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::{Backlog, PollBudget, RunAffinity, Scratch, Shares, Sink};
use crate::reserve::MachineBudget;

impl Backlog {

    #[must_use]
    pub fn with_affinity(self, affinity: RunAffinity) -> Backlog {
        Backlog { affinity, ..self }
    }

    #[must_use]
    pub fn new() -> Backlog {
        Backlog::default()
    }

    #[must_use]
    pub fn with_window(window: PollBudget) -> Backlog {
        Backlog {
            table: std::sync::Arc::default(),
            scratch: Scratch::open(),
            window,
            sink: None,
            shares: Shares::Unset,
            affinity: RunAffinity::Os,
            budget: MachineBudget::unmetered(),
        }
    }

    #[must_use]
    pub fn with_sink(self, sink: Sink) -> Backlog {
        Backlog {
            sink: Some(sink),
            ..self
        }
    }
}
