// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One session: the four questions a person arrives with, and five readings of what it did.
//!
//! Index only.

mod facts;
mod links;
mod page;
mod tabs;
#[cfg(test)]
mod tests;

pub use facts::{Fact, head_facts};
pub use links::{building_of, room_for_link};
pub use page::SessionView;
pub use tabs::Tab;
