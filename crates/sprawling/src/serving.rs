// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a served city is made of before its writer exists, and what it
//! reaches the outside through, as opposed to the writer itself.
//!
//! Five things live here: the key this listener will present at its
//! door, settled before a socket exists; the vault, opened and asked what
//! it really is; the [`Serving`] value one caller fills in; the process
//! log's way out ([`Journal`]); and the fold that keeps the views beside
//! the writer. The writer thread, the desk commands wait on and the lanes
//! that drive runs are the assembly point's (`bin::assembly`), which
//! consumes all of this; nothing here names it back (sprawling-SPEC.md
//! 8-92).
//!
//! Randomness is drawn here rather than in `bin::keying`, which is pure:
//! this crate draws entropy in one place, and a key a third party can
//! predict is a door a third party can open.

pub(super) mod door;
pub(crate) mod folding;
pub(crate) mod journal;
pub(super) mod serve;
#[cfg(test)]
mod tests;

pub(crate) use door::random_token;
pub use door::{Keyed, key_for, open_vault};
pub use journal::Journal;
pub use serve::Serving;
