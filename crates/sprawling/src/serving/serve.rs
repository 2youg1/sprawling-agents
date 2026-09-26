// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The value a serve travels on: [`Serving`], everything one served
//! city is made of.
//!
//! Data only. Nothing here binds, spawns or writes; `assembly::listen`
//! consumes [`Serving`] and splits the writer thread's share of it off
//! as `assembly::attending::Opening`. Keeping the shape apart from the
//! code that acts on it is what lets one caller settle every field
//! before any thread exists.
//!
//! [`Serving`] is public, so every field of it is part of what an
//! embedder of this library writes, and a field added to it breaks them.

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
