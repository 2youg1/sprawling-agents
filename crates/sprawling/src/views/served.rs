// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a served city hands its views that the ledger cannot.
//!
//! Everything else the views hold is folded from records, and a rebuild
//! reproduces it byte for byte. These two are facts about the running
//! process rather than about the history - what this machine had when
//! the city started, and the vault the worker opened - so they arrive
//! from outside, once, and a rebuild leaves them alone.
//!
//! They are here rather than in `views::holding` because that module's
//! subject is what a fold holds; a setter that no fold can reach is a
//! different responsibility, and one file holding both is what makes
//! the distinction easy to lose.

use super::holding::Views;

impl Views {
    /// Takes what the doctor found, so a page can be told what this
    /// machine is missing.
    ///
    /// Probing is seconds of starting programs, and a read that did it
    /// would hold the one thread every other read is answered on.
    pub(crate) fn found_on_this_machine(&mut self, report: channels::DoctorAnswer) {
        self.machine = Some(report);
    }

    /// Takes the vault handle the worker opened, for the one read that
    /// needs a credential: reaching a tool server whose header or
    /// environment names one.
    ///
    /// Lent rather than opened here, for the reason the transcription
    /// door is lent it: a second handle on the same secrets would be a
    /// second door onto them.
    pub(crate) fn lend_the_vault(
        &mut self,
        vault: std::sync::Arc<std::sync::Mutex<gateway::Custodian>>,
    ) {
        self.vault = Some(vault);
    }
}
