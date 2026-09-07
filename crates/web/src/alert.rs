// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The only module that may interrupt a person, and only once per fact.
//!
//! Index only.

mod judge;
mod notify;
#[cfg(test)]
mod tests;

pub use judge::{Alert, AlertKind, Alerts, Raise, Refused, absorb, alert_for, cleared_by, refused};
#[cfg(target_arch = "wasm32")]
pub use notify::{ask_to_interrupt, interrupt};
