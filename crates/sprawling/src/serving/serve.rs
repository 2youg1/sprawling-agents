// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The two values a serve travels on: [`Serving`], everything one
//! served city is made of, and [`Opening`], everything the writer
//! thread is opened with.
//!
//! Data only. Nothing here binds, spawns or writes; `serving::worker`
//! consumes [`Serving`] and `serving::attending` consumes [`Opening`].
//! Keeping the shapes apart from the code that acts on them is what
//! lets one caller settle every field before any thread exists.
//!
//! The point a reader most often gets wrong: the two values face
//! opposite ways. [`Serving`] is public, so every field of it is part
//! of what an embedder of this library writes and a field added to it
//! breaks them; [`Opening`] is visible only inside `serving`, so the
//! writer thread's parameters may be reshaped freely.

use std::net::SocketAddr;

use kernel::Payload;

/// Everything one served city is made of, in one value.
///
/// Eight loose parameters is a signature nobody calls correctly from
/// memory, and two `Option`s of the same shape passed the wrong way
/// round is a mistake the compiler cannot see. Named fields make the
/// call site say which is which.
pub struct Serving {
    pub city_root: std::path::PathBuf,
    pub addr: SocketAddr,
    /// The pairing token in plaintext, read once by the caller. It gets
    /// no further than the digest this takes from it, except into the
    /// console's `/web`, which is the one place it has to travel.
    pub token: Option<String>,
    pub client: channels::ClientAssets,
    pub vault: gateway::Custodian,
    pub vault_notice: Option<Payload>,
    pub log: runtime::diagnostics::Diagnostics,
    /// The other end of the sink `log` was built with. It travels
    /// beside the log rather than being made here, because the sink has
    /// to exist before the `Diagnostics` does and a second channel made
    /// here would carry nothing.
    pub journal: crate::serving::Journal,
    pub console: Option<crate::console::Terminal>,
}

/// What a worker is opened with: where the city is, whose keys it may
/// redeem, what the vault turned out to be, and where its diagnostics
/// go.
///
/// Four values that always travel together and are never chosen
/// independently - `serve` settles all four before it has a thread to
/// hand them to - so they travel as one, as `Reporter` does.
pub(super) struct Opening {
    pub(super) city_root: std::path::PathBuf,
    pub(super) vault: gateway::Custodian,
    /// What the vault probe found, on its way to the ledger as a
    /// disclosure. Consumed by the first `open_for_service`.
    pub(super) notice: Option<Payload>,
    pub(super) log: runtime::diagnostics::Diagnostics,
}
