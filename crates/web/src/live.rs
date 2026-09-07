// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Watching one session as it happens, in a window that says what it dropped.
//!
//! Index only.

mod commands;
mod composer;
mod describe;
mod feed;
mod page;
mod rounds;
mod stream;
#[cfg(test)]
mod tests;

pub use commands::{cancel_command, fork_command, short_run};
pub use describe::{Changed, describe, describe_in};
pub use feed::{Feed, Line, WINDOW};
pub use page::LiveView;
