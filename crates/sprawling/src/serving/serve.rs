// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How a city is stood up and served, as opposed to how one piece of
//! work is run.
//!
//! Four things happen here and nothing else: the key this listener will
//! present at its door is settled before a socket exists, the vault is
//! opened and asked what it really is, the one writer thread is started
//! with the ledger inside it, and the socket is handed the four sinks it
//! may reach the city through.
//!
//! **The writer thread is the city's one writer.** The ledger is opened
//! inside it and never leaves, so the type never has to cross a thread
//! boundary to prove that a city has one writer (ARCHITECTURE section
//! 10). Everything a socket does reaches it as a `Command` on a desk,
//! one at a time.
//!
//! Randomness is drawn here rather than in `bin::keying`, which is pure:
//! this crate draws entropy in one place, and a key a third party can
//! predict is a door a third party can open.

use std::net::SocketAddr;

use kernel::Payload;

/// A URL-safe random string of `bytes` bytes of OS entropy.
///
/// Deliberately not the simulator's seeded randomness: a verifier a
/// third party can predict is a login a third party can finish. This is
/// the one place in the binary where reproducibility would be a defect.
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
    pub console: Option<crate::console::Terminal>,
}

/// Starts the city's one writer, and returns once it is running.
///
/// The ledger is opened *inside* this thread and never leaves it: a city
/// has one writer, and the type never has to cross a thread boundary to
/// prove it. The handshake is part of the contract - a thread that comes
/// back from this function has already opened the history and said so,
/// so a caller never serves a socket over a city that failed to open.
///
/// # Errors
/// Propagates whatever opening the history reports, a thread the
/// platform will not start, and a worker that ended before reporting.
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
