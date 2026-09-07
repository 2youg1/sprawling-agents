// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The tabs: five readings of what a session did.

use crate::lang::Msg;

/// Which part of a session is being read.
///
/// Tabs rather than four panels down one page: they are four readings of
/// one session and only one is wanted at a time, so stacking them makes
/// a person scroll past three answers to reach the one they came for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Turns,
    Changes,
    Cost,
    Docs,
    /// What was sent, as opposed to what came back. Last because it is
    /// the one a person opens on purpose rather than on arrival.
    Prompt,
}

impl Tab {
    /// Every tab, in reading order: what it did, what that changed, what
    /// it cost, what it wrote down, and what it was sent.
    pub const ALL: [Tab; 5] = [Tab::Turns, Tab::Changes, Tab::Cost, Tab::Docs, Tab::Prompt];

    /// What the tab is called.
    #[must_use]
    pub fn word(self) -> Msg {
        match self {
            Self::Turns => Msg::SessionTabTurns,
            Self::Changes => Msg::SessionTabChanges,
            Self::Cost => Msg::SessionTabCost,
            Self::Docs => Msg::SessionTabDocs,
            Self::Prompt => Msg::SessionTabPrompt,
        }
    }
}
