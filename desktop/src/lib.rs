// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! An MCP server that gives an agent eyes and hands on this Windows
//! desktop.
//!
//! Read `desktop/README.md` for what it is and `desktop/desktop-SPEC.md`
//! for why it has this shape. The whole public surface is one function,
//! because everything else about a server is reached through its
//! protocol.

mod platform;
mod refusal;
mod rpc;
mod scope;
mod session;
mod tools;

pub use session::serve_stdio;
