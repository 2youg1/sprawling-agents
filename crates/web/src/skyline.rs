// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a city of buildings looks like: height from assets, a lit band from the plan.
//!
//! Index only.

mod faces;
mod prisms;
#[cfg(test)]
mod tests;

pub use faces::{DisplayList, done_band_of, draw, faces_of, windows_of};
pub use prisms::{Prism, face_tokens, painter_order, place, prisms_of, storeys, unreadable_rows};
