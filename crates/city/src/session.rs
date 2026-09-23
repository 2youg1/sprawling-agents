// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A session starts here: the shape the last one froze is forgotten, and
//! the handoff slot is cleared or kept.
//!
//! **Why this exists at all.** A session's first run writes the model it
//! calls and the effort it thinks with into the room's own
//! `CONFIG.toml`, and every later run in that room refuses to move them,
//! because a provider caches a conversation's prefix only for as long as
//! the shape of the calls behind it stays put (`city-SPEC.md` 8-14).
//! That refusal is right, and until now it had no counterweight: a person
//! who changed the model could no longer dispatch into that room at all.
//! Starting a new session is that counterweight.
//!
//! **Two files, one decision.** What a new session inherits is a single
//! question, so it is answered in a single call: the frozen shape is
//! always forgotten, and the handoff — what the last session in this room
//! wrote for the next one — is kept only when the person asked to carry
//! it. Two calls would let a caller perform the half it remembered and
//! leave the other in place, and the two half-states are both lies: a
//! room that kept its shape still cannot change model, and a room that
//! kept its handoff says it starts fresh while feeding the next run the
//! previous session's summary.
//!
//! **The handoff slot is emptied, not annotated.** The file is removed
//! rather than replaced by the blank form `city::spine_files` lays down
//! when a room is opened: `handoff` answers `None` for both, and one
//! state is fewer than two. Nothing is lost by removing it, because the
//! bytes are in the ledger under `handoff_written` and a replay reads
//! them back from there.
//!
//! **Nothing here decides whether a handoff exists.** `city::handoff`'s
//! own rule — no file, or a form still holding the template's guidance,
//! is `None` — stays the only authority on that; this module only clears
//! the slot.

use std::path::Path;

use kernel::{Address, AxError};

use crate::config_layers;
use crate::spine_files;

/// Begins a session at this address: the next run there chooses its own
/// shape and carries nothing.
///
/// The two halves are [`forget_shape`] and
/// [`crate::spine_files::clear_handoff`], and this is the call that keeps
/// them together: it is what `Command::OpenSession` with
/// `Carry::Nothing` performs (`sprawling-SPEC.md` 8-82).
///
/// # Errors
/// Propagates a room configuration that exists and cannot be read or
/// written — a file this build cannot understand is not one to overwrite
/// — and a handoff slot that exists and cannot be removed.
pub fn clear_session(city_root: &Path, addr: &Address) -> Result<(), AxError> {
    forget_shape(city_root, addr)?;
    spine_files::clear_handoff(city_root, addr)
}

/// Begins a session that carries the handoff: the shape goes, the
/// previous session's summary stays.
///
/// The half `Carry::Handoff` needs and the reason it is not
/// [`clear_session`] itself: a person who asks to carry the summary is
/// still starting a new session, so the model and the effort must be
/// choosable again — that is what made the room dispatchable. What they
/// asked to keep is the one thing this leaves alone.
///
/// # Errors
/// Propagates a room configuration that exists and cannot be read or
/// written.
pub fn forget_shape(city_root: &Path, addr: &Address) -> Result<(), AxError> {
    config_layers::forget_session(city_root, addr)
}

#[cfg(test)]
mod tests;
