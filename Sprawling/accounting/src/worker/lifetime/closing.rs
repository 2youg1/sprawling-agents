// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::{RunWorker, city_segment};
use kernel::{AxError, EventKind, Locator};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Closing {
    Chosen { by: ClosedBy, mode: wire::CloseMode },

    Broken { cause: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClosedBy {
    Console,

    Page,

    InterruptSignal,

    BreakSignal,

    Terminate,

    TerminalLost,
}

impl ClosedBy {
    fn said(self) -> &'static str {
        match self {
            Self::Console => "with /quit in its terminal",
            Self::Page => "from a page on this machine",
            Self::InterruptSignal => "by an interrupt signal",
            Self::BreakSignal => "by Ctrl+Break",
            Self::Terminate => "by a terminate signal",
            Self::TerminalLost => "because its terminal went away",
        }
    }
}

impl RunWorker {
    pub(crate) fn close_city(&mut self, why: &Closing) -> Result<(), AxError> {
        let mut must_read = Vec::new();
        let bytes = city_segment(&self.city_root)?.bytes;
        let hash = self
            .cas
            .put(&bytes)
            .map_err(storage::StorageError::into_ax)?;
        must_read.push(Locator::cas(hash));
        let standing = self.ledger.position();
        let (overview, context, next_step) = match why {
            Closing::Chosen { by, mode } => (
                format!("the city was closed {}", by.said()),
                match mode {
                    wire::CloseMode::Drain => {
                        "an orderly close, not a crash: every run under way landed before this line"
                    }
                    wire::CloseMode::Interrupt => {
                        "an interrupted close, not a crash: runs still under way were stopped at their next safe point"
                    }
                }
                .to_owned(),
                "`sprawling serve` on this directory continues from here".to_owned(),
            ),
            Closing::Broken { cause } => (
                format!("the city stopped because serving failed: {cause}"),
                "not a choice: serving failed; the ledger holds every line that landed before it"
                    .to_owned(),
                "fix what the failure names, then `sprawling serve` on this directory".to_owned(),
            ),
        };
        let handoff = runtime::handoff::Handoff::new(
            must_read,
            overview,
            format!("the ledger stands at {}", standing.value()),
            context,
            next_step,
        )?;
        self.note(
            runtime::diagnostics::Level::Effect,
            "accounting::worker",
            "the city is closing; its handoff is on the ledger",
        );
        self.record(EventKind::HandoffWritten, handoff.payload()?)
    }
}
