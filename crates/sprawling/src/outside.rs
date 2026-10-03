// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The remote door, wired into this binary (`crates/sprawling/Spec.lean` §8-139,
//! §8-140): how a person away from this machine reaches the city through
//! a route, and the console verbs that open the way.
//!
//! `remote_access` decides who may enter and holds the cryptography; it
//! knows no frame and opens no port. What only this binary can do lives
//! here: the door kept behind one lock with its ledger lines and device
//! table (`keeper`, `devices`), the loopback listener the route makes
//! reachable (`listener`), each session's frames judged by their class
//! (`conduit`, `verbs`), and `/remote` on the console (`console`).
//!
//! Named `outside` rather than `remote`, which a `use` would read as the
//! external crate as well, and rather than `relay` or `door`, which this
//! binary already gives to the ledger crossing and to its own listener's
//! key.

pub(crate) mod asking;
mod conduit;
pub(crate) mod console;
mod devices;
pub(crate) mod keeper;
pub(crate) mod listener;
mod verbs;

#[cfg(test)]
mod tests;
