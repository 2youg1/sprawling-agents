// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a served city says to the terminal it runs in, and what a line
//! typed there means (`crates/sprawling/spec/Console.lean` §8-11).
//!
//! The console has two faces, the CLI and the quiet host, and a third
//! for a pipe, the line console; which one shows is the lifecycle's
//! (`lifecycle`), what is drawn is the one writer's (`ui`), and what a
//! line does is the session's (`cli`), so the console decides nothing
//! the server does not.

pub(super) mod cli;
pub(super) mod editor;
pub(crate) mod language;
pub(crate) mod lifecycle;
pub(super) mod program_status;
pub(super) mod screen;
pub(super) mod stream;
pub(super) mod terminal;
pub(super) mod ui;
pub use lifecycle::Surface;
pub use terminal::Terminal;
pub(crate) use terminal::{Answering, Inside, start};
#[cfg(test)]
mod tests;
