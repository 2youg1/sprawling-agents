// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The listening end, and the humble half of it (ARCHITECTURE section
//! 9). Every branch here is a send, a receive, or the end of a session;
//! the judgements it applies are `channels::reception`'s and the bytes
//! it serves are `channels::assets`'.
//!
//! Five jobs and no policy: serve the client bundle, upgrade a
//! WebSocket, accept an upload, take a credential from a caller on this
//! machine, and let an outside editor drive the city.
//!
//! A refusal made minutes later has no way home, which is why a command
//! carries the [`Reply`] address of whoever sent it.

mod config;
mod reply;
mod socket;

pub use config::{AcpBody, AcpProgress, AcpSink, ServeConfig, router};
pub use reply::{Delivered, Reply};
pub use socket::serve;
