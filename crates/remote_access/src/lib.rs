// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Remote access: who may reach a city from outside its machine, and
//! until when (crates/remote_access/Spec.lean). The door that decides it is
//! [`door::Door`], whose properties the `RemoteDoor` model states.
//!
//! A route that makes the door reachable carries bytes and nothing else,
//! so every decision here is taken on the city's own machine and none of
//! the door's decisions names a route; [`route`] holds the routes
//! themselves. A caller reaches a decision through the module that owns
//! it, so the module name says which authority answered.

pub mod door;
pub mod handshake;
pub mod keys;
pub mod pairing;
pub mod route;
pub mod seal;
