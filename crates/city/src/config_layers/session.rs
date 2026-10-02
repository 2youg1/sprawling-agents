// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a session froze at its own address: the model it calls and, when
//! a person chose one, how hard it thinks.
//!
//! **The address's own `CONFIG.toml` is the record.** The ladder that
//! answers what governs a run stays where it is: the model still comes
//! from the endpoint book and the effort still climbs city -> building
//! -> room. This module answers the narrower question a session needs —
//! *what did this address write down for itself* — and the difference is
//! the point, because a value inherited from the city or the building is
//! a default this address never chose and cannot be the record of a
//! choice.
//!
//! **One read, one write, and the write happens once.** The first run in
//! a session writes the shape it froze; every later run reads it back
//! and refuses to move it, because a provider caches a conversation's
//! prefix for as long as the model and the effort behind it stay put
//! (`crates/sprawling/Spec.lean` §8-79 does the refusing). An address that is its
//! own building has one file for both rungs, so that address is its own
//! session.
//!
//! The write goes through `write::change`, the one write path into a
//! `CONFIG.toml`: a second path would be a second answer to what "whole
//! or not at all" means.
//!
//! Specified by `crates/city/spec/ConfigLayers.lean` §8-14.

use std::path::Path;

use kernel::{Address, AxError, B3Hash, Effort};

use super::write::{Change, change};
use super::{ConfigLayer, Layer, path};

/// What the configuration at `addr` itself states, with the ladder
/// above it left out.
///
/// # Errors
/// Refuses an unreadable file and a file that does not parse, exactly
/// as [`super::load`] does. A missing file states nothing.
pub fn own_layer(city_root: &Path, addr: &Address) -> Result<ConfigLayer, AxError> {
    super::ladder::stated(&path(city_root, addr, Layer::Resident)?, Layer::Resident)
}

/// Records what a session froze at its own address.
///
/// One act rather than two, because the two values are one choice: the
/// model a session opened with and the effort it opened with are frozen
/// together, and a record that could hold one without the other would
/// let a later run read half a shape.
///
/// The model is written even when the person chose none, and the effort
/// only when they did: a model always names something, while an absent
/// effort is the provider's own decision rather than a value, and a file
/// that spelled it would record a choice nobody made.
///
/// # Errors
/// Propagates what the write path reports: a file that exists and cannot
/// be read or parsed — a configuration this build cannot understand is
/// not one to overwrite — and a directory that cannot be written.
pub fn write_session(
    city_root: &Path,
    addr: &Address,
    model: &str,
    effort: Option<Effort>,
) -> Result<(), AxError> {
    change(
        city_root,
        addr,
        Layer::Resident,
        Change::Session { model, effort },
    )
}

/// Records the identity version a session froze at its own address
/// (`crates/city/spec/Identity.lean` §8-33).
///
/// Written by the session's first run, beside the shape it froze, and
/// taken out with that shape by [`forget`].
///
/// # Errors
/// Propagates what the write path reports, as [`write_session`] does.
pub fn freeze_naming(city_root: &Path, addr: &Address, version: B3Hash) -> Result<(), AxError> {
    change(city_root, addr, Layer::Resident, Change::Naming(version))
}

/// Forgets the record, so the next session at this address chooses its
/// own shape.
///
/// The inverse of [`write_session`], and one act for the same reason:
/// the model and the effort are one choice, and forgetting one of them
/// would leave a later run reading half a shape it cannot move. Only the
/// keys this module wrote are removed, because the file is a person's as
/// well as a session's (`crates/city/spec/Session.lean` §8-14b).
///
/// # Errors
/// Propagates a file that exists and cannot be read or parsed, and a
/// directory that cannot be written.
pub(crate) fn forget(city_root: &Path, addr: &Address) -> Result<(), AxError> {
    change(city_root, addr, Layer::Resident, Change::Forget)
}

#[cfg(test)]
mod tests;
