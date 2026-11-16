// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

pub(crate) mod cli;
pub(crate) mod host;
pub(crate) mod language;
pub mod lifecycle;
pub(crate) mod local_time;
pub mod mei;
pub(crate) mod stream;
pub(crate) mod terminal;
pub use lifecycle::Surface;
pub use terminal::{Answering, CarryRemote, Clock, Inside, OpenPaired, Terminal, start};
#[cfg(test)]
mod tests;
