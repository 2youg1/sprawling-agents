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

/// A URL-safe random string of `bytes` bytes of OS entropy.
///
/// Deliberately not the simulator's seeded randomness: a verifier a
/// third party can predict is a login a third party can finish. This is
/// the one place in the binary where reproducibility would be a defect.
pub(super) mod attending;
pub(crate) mod desk;
pub(super) mod door;
pub(crate) mod pool;
pub(crate) mod relay;
pub(super) mod serve;
#[cfg(test)]
mod tests;
pub(super) mod worker;

pub(crate) use desk::{CommandDesk, Posted};
pub(crate) use door::random_token;
pub use door::{Keyed, key_for, open_vault};
pub use serve::Serving;
pub use worker::serve;
