// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! City export and restore: the backup that can be carried to another
//! machine, and the reading of it back.
//!
//! What travels is the history, the objects its locators point at, and
//! the files a person works in. What does not travel is anything the
//! ledger can rebuild — projections and indexes — because a second copy
//! of a derived view is a second statement of what happened.
//!
//! Credentials never travel, and not by omission: they are not in the
//! city to begin with. They live in the host machine's vault, so a
//! bundle that reached the wrong hands carries work, not access.
//!
//! A bundle is a directory rather than one file. A single file would
//! need either a container format of our own to maintain or a
//! compression dependency to carry; a directory needs neither, and any
//! backup tool can wrap one.

mod export;
mod files;
mod manifest;

pub use export::Bundle;
pub use files::open_restored;
pub use manifest::{MANIFEST, Manifest};
