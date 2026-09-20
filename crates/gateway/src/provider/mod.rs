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
//! The table is a data plane: every row carries the vendor page or the
//! measurement it came from, and a fact no row states is left unstated
//! rather than invented. A figure this city made up would outrank the
//! one that bills.

pub mod ceiling;
pub mod preset;
