// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The four things only a browser has, and nothing that decides.
//!
//! Index only.

mod address;
mod frame;
mod keys;
mod outbound;
mod shell;
mod wiring;

pub(crate) use address::follow_the_address_bar;
pub(crate) use keys::{Keyboard, listen_for_keys};
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use outbound::Outbound;
#[cfg(target_arch = "wasm32")]
pub(crate) use shell::connect;
#[cfg(target_arch = "wasm32")]
pub(crate) use wiring::Wiring;
