// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one translation between a View and the address bar, both ways.
//!
//! Index only.

mod fragments;
mod places;
#[cfg(test)]
mod tests;
mod view;

#[cfg(target_arch = "wasm32")]
pub use fragments::{current, go, unresolved};
pub use fragments::{from_fragment, to_fragment};
#[cfg(target_arch = "wasm32")]
pub(crate) use places::place_view;
pub use places::{Destination, destinations, opened_building, showing};
pub use view::{Lens, View};
