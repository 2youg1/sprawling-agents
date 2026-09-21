// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a session already froze: the model it calls and how hard it
//! thinks, chosen by its first run and kept by every run after it.
//!
//! A session is a room, and a provider caches a conversation's prompt
//! prefix for as long as the shape of the calls in it does not move.
//! The model, its output ceiling and the thinking effort all reach the
//! wire, so any of them changing halfway makes every later turn pay
//! full price for a prefix whose bytes never moved. The city therefore
//! writes down what its first run froze, in the room's own
//! `CONFIG.toml`, and reads it back at every dispatch into that room.
//!
//! **The room's file is the record, not a second setting.** The model a
//! run calls is still chosen by the endpoint book, and the effort still
//! climbs the city -> building -> room ladder; both are the authority
//! for where a session *starts*. What the room states is what that
//! session *started with*, so a registry that moved since is a
//! disagreement this dispatch refuses instead of a change it makes
//! silently. A refusal names the two ways out, because both of them
//! keep the frozen prefix: open a new session, or fork this run.

use kernel::{AxError, Effort};
use runtime::turn::CallShape;

use super::super::{Assignment, RunWorker};

#[cfg(test)]
mod tests;

impl RunWorker {
    /// Records the shape this dispatch freezes, or refuses a dispatch
    /// that would move the shape the session already froze.
    ///
    /// Called once per dispatch, after the room exists and before
    /// anything else is written for the run, so a refusal costs the
    /// person no file and no record. A session's first run is the one
    /// dispatch allowed to choose: it writes the model it chose and the
    /// effort it was given into the room's own layer.
    ///
    /// Every later run in that room reads the record back and compares
    /// the shape it would freeze against it — through
    /// [`CallShape::verified_against`], which owns both the comparison
    /// and the words of the refusal. The effort is compared as the
    /// ladder resolves it here, because an effort stated nowhere is the
    /// provider's decision rather than a value, and moving from "let the
    /// provider decide" to a level is as much a moved shape as moving
    /// between two levels.
    ///
    /// A model registered twice under two tags is one model: the book's
    /// entry for it is found by id, and the ceiling and the window are
    /// read from that entry rather than recorded here. They are
    /// properties of the model, so a copy of them in this file would be
    /// a second answer to what the book already answers.
    ///
    /// # Errors
    /// Propagates the room's own configuration failing to read or
    /// write, and refuses `E_CONFIG_INVALID` for a dispatch that would
    /// move the model, the ceiling or the effort the session froze.
    pub(super) fn choose_shape(
        &mut self,
        at: &Assignment,
        model: &gateway::ModelEntry,
    ) -> Result<(), AxError> {
        let own = city::own_layer(&self.city_root, &at.addr)?;
        let Some(frozen_model) = own.model() else {
            return city::write_session(&self.city_root, &at.addr, &model.id, at.effort);
        };
        // What the session froze. The effort is the ladder's answer at
        // this address, which is the value the run that opened the
        // session froze and the value this one would freeze.
        let settled = city::settled_effort(&self.city_root, &at.addr)?.map(|(effort, _)| effort);
        let frozen = self.frozen_shape(frozen_model, settled);
        let moved = CallShape {
            model: model.id.clone(),
            max_tokens: model.max_output_tokens,
            effort: at.effort.or(settled),
            context_tokens: model.context_tokens,
        };
        // The comparison and the words of the refusal are the runtime's,
        // so that a shape moving mid-session is refused once and in one
        // sentence wherever it is noticed.
        moved.verified_against(&frozen)
    }

    /// The shape the session froze, as the endpoint book states it now.
    ///
    /// A model the book no longer holds answers with no ceiling and no
    /// window. That is not a guess: the comparison refuses on the model
    /// before it reaches either of them, and a value invented here
    /// would be a fact this city does not have.
    fn frozen_shape(&self, model: &str, effort: Option<Effort>) -> CallShape {
        let held = self
            .book
            .choices()
            .find(|(_, _, entry)| entry.id == model)
            .map(|(_, _, entry)| entry);
        CallShape {
            model: model.to_owned(),
            max_tokens: held.and_then(|entry| entry.max_output_tokens),
            effort,
            context_tokens: held.map_or(0, |entry| entry.context_tokens),
        }
    }
}
