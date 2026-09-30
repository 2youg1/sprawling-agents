// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Main's assembly point: what only this binary can hand the city's one
//! writer. The writer itself is `accounting::worker::RunWorker`; this is
//! the dirtiest component and the only omniscient one, because it knows
//! every concrete type and nothing knows it (accounting-SPEC.md 12-12).
//!
//! The wall clock is sampled in `production` only (determinism rule 2),
//! and a worker receives it, with every other hand it reaches this
//! machine through, as one `Hands` value made there. The port is taken
//! and the writer opened in `listening`; the writer's thread is started
//! in `attending`, the background chain audit in `chain_watch`, and a
//! file dropped onto the composer is kept by `dropping`.

mod attending;
mod chain_watch;
mod dropping;
mod listening;
mod production;

pub use listening::{Listening, listen};
pub use production::{SystemClock, form_city, hands, init_city};
