// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

pub mod assembly;
pub mod audience;
pub mod beside_binary;
pub use calyx::console;
pub mod doctor;
#[cfg(windows)]
pub mod environment_broadcast;
pub mod firstrun;
pub mod monitor;
pub mod privacy;
pub mod release;
mod revealing;
pub mod serving;
pub mod supervising;

mod browser_bidi;
mod browser_tool;
mod keying;
mod outside;
