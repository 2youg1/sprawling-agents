// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Everything a client may ask the city to do, and the two invariants
//! that live in the type system rather than in a check.
//!
//! - `Command` is generic over the carrier of a secret. `WireCommand`
//!   fixes that carrier to an uninhabited type, so a frame arriving from
//!   a socket cannot be a `PutSecret` — not "is rejected", but has no
//!   representation. Credentials are enrolled on the host machine, and
//!   that constraint is held by construction.
//! - Every state-changing Command owns an `IdemKey` field. There is no
//!   constructor that omits it, so "double-clicking twice opens two
//!   runs" is not reachable from this type.
//!
//! The enum is not `#[non_exhaustive]`: the wire version is the
//! versioning mechanism, so the assembly layer must handle every variant
//! and a new one fails to compile until somebody decides what it does.
//! That is what keeps a button off the client until the city can answer
//! the frame behind it.

mod kind;
mod wire;

pub use kind::PursuitStep;
pub use kind::{COMMAND_NAMES, Command, GovernedDocument, HaltScope, LoginStep, NoSecret};
pub use wire::WireCommand;
