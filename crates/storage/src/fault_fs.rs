// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! FaultFs: the second Vfs adapter — a deterministic power-loss model
//!. Compiled for tests and for citysim under the
//! `fault` feature; never in a production build.
//!
//! The model is stricter than any real platform so the write discipline
//! it enforces holds on every platform (`crates/storage/spec/FaultFs.lean` §8-2):
//! - every file has two planes: `durable` (survives power loss) and
//!   `live` (what the running process observes). `sync_data` promotes
//!   live to durable; a power cut drops the unsynced delta, except a
//!   `TornTail` prefix that models a torn write reaching the platter.
//! - a created file's directory entry survives only after `sync_dir` on
//!   its parent — even when its bytes were synced (stricter than POSIX).
//! - removals are instantly durable (simplification: the only remover is
//!   tail recovery, and a resurrected empty segment is harmless — reopen
//!   tolerates it).
//! - the op hitting `cut_at_op` fails with "power lost"; the plan is then
//!   consumed, so the same instance serves the powered reopen. Appends
//!   land on `live` before the cut check so the tear can bite the very
//!   write that died.
//!
//! Everything is explicit in [`FaultPlan`]; there is no randomness.

mod fs;
mod plan;

pub use fs::FaultFs;
pub use plan::{FaultPlan, TornTail};
