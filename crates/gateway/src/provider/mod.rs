// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What this city knows about a provider before it has asked one.
//!
//! Two questions live here. *What does this host serve, and where* —
//! the path it answers under, the shape it answers in, and the ceilings
//! of the models whose names it prints in its own documentation
//! (`preset`). *Which statement about an output ceiling wins* — the
//! ladder from the person's own figure down to the city's policy
//! default (`ceiling`).
//!
//! *How one attached endpoint is connected* — resolved once at attach
//! and recorded, so that no call path derives it again (`registry`),
//! together with the faces beyond a conversation that connection
//! serves (`modality`).
//!
//! The table is a data plane: every row carries the vendor page or the
//! measurement it came from, and a fact no row states is left unstated
//! rather than invented. A figure this city made up would outrank the
//! one that bills.

pub mod ceiling;
// Both are re-exported by the crate root, so the attach path in
// `bin::assembly` reaches them by name the day `WIRE_V` carries
// `connection_kind` (roadmap 4.5).
pub mod modality;
pub mod preset;
pub mod registry;
/// The guard on the system prefix this city sends (roadmap 17.2).
#[cfg(test)]
mod stability;
