// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The console's renderer: a Zig leaf that turns a scene into the bytes
//! of one terminal frame, behind a `(ptr, len)` boundary and offered as
//! one safe function (`crates/console_ffi/Spec.lean`).
//!
//! The console (`bin::console` in the `sprawling` crate) decides what
//! the screen holds; the leaf decides how it looks. The only `unsafe` in
//! this package is the one call into the leaf, in `leaf`, with the
//! precondition that makes it sound written beside it.

mod leaf;
pub mod part;
pub mod scene;

pub use leaf::{Frame, Refused, draw};

#[cfg(test)]
mod tests;
