// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The only place in this crate that talks to the server.
//!
//! Index only.

mod enrol;
mod frames;
mod link;
#[cfg(test)]
mod tests;

pub use enrol::{Enrolment, enrol};
#[cfg(target_arch = "wasm32")]
pub use frames::{pairing_token, socket_url};
pub use frames::{read_frame, token_in};
pub use link::{Link, LinkAction, LinkEvent, LinkState, backoff_ms};
#[cfg(target_arch = "wasm32")]
pub use link::{open, send};
