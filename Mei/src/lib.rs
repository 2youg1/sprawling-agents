// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Archetype:Mei's three production faces: the serial agent loop, the
//! Anthropic transport, and the persistent Python adapter (`Mei/Spec.lean`).

pub mod agent;
pub mod error;
pub mod python;
pub mod transport;

pub use error::{Error, Result};
