// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a line typed into a serving city means (`crates/sprawling/spec/Console.lean`
//! §8-11).
//!
//! The terminal `sprawling up` blocks in is a surface of its own: on a
//! machine with no browser it is the only surface there is.
//!
//! Everything here is a pure judgement over one line of text. What the
//! judgement produces is either a control action the terminal carries
//! out or a `ClientFrame` that goes to the same desk a browser's frames
//! go to, so the console decides nothing the server does not.

/// The console's own verbs, which are not on the wire.
///
/// Exhaustive, and checked against the wire's vocabulary so a name can
/// never mean two things. `serving` rather than `status`: the glossary
/// already gives `status` to the tool that answers what one run's
/// situation is, and one name per concept is a gate.
/// `AttachEndpoint` becomes `attach_endpoint`.
///
/// The wire names itself in the shape Rust variants take; a terminal is
/// typed in lower case. One conversion, so the two spellings cannot
/// become two lists.
pub(super) mod language;
pub(super) mod terminal;
pub use terminal::Terminal;
pub(crate) use terminal::{Answering, Inside, start};
#[cfg(test)]
mod tests;
