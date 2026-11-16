// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

pub(crate) mod browsers;
pub(super) mod door;
pub(crate) mod folding;
pub(crate) mod journal;
pub mod key_file;
pub(crate) mod output_ring;
pub(crate) mod placement;
pub(super) mod serve;
pub mod standing;
#[cfg(test)]
mod tests;

pub use door::{Keyed, key_for, open_vault};
pub use journal::{Clock, Journal};
pub use serve::Serving;
pub use standing::{serving_runtime, setting_telling_a_refusal};
