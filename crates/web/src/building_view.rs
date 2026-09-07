// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One building, what it has written down, and what waits in each room.
//!
//! Index only.

mod faces;
mod leaf;
mod page;
mod room;
#[cfg(test)]
mod tests;
mod text;

pub use leaf::{Leaf, opening_leaf, room_addr};
pub use page::BuildingView;
pub use room::{RoomQueue, day_label, waiting_in};
