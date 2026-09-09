// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Statistical evidence: suites, holdouts, probes, asset scoring,
//! metabolism. Never a merge gate (C11).

// The City.md ablation instrument answers on demand through its own
// ignored test and has no product caller; compiling it into the shipped
// library would ship an instrument, not a capability (eval-SPEC 8-7).
#[cfg(test)]
mod ablation;
mod metabolism;
mod nesting;
mod probe;
mod score;
mod suite;

pub use metabolism::{ASSET_FLOOR_PER_MILLE, ASSET_IDLE_DAYS, Disposal, dispose, sweep};
pub use nesting::{Attempt, Fault, Grades, Shape, Verdict, grade, recommended, tally};
pub use probe::{Answers, Comparison, Probe, ProbeId, compare, handoff_probe};
pub use score::{AssetUse, Score, score, worst_first};
pub use suite::{Half, Outcome, Report, Suite, Tally, Task};
