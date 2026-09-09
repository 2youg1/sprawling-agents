// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Starting an engine that speaks WebDriver BiDi, and the socket its
//! frames cross.
//!
//! Firefox is the first engine because it speaks the protocol itself: a
//! machine with Firefox on it is a machine this tool works on, with
//! nothing downloaded. Chromium is reachable only through
//! `chromedriver`, so it is a second road rather than a second lane.
//!
//! Which programs exist on this machine is not decided here. The engine
//! is started by name and the operating system resolves it, because
//! `bin::doctor` already owns the question of what is installed and a
//! second answer to it would drift.

mod engine;
mod lazy;
mod socket;

pub(crate) use lazy::{LazyEngine, port_for};
