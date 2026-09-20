// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Statistical evidence: suites, holdouts, probes, asset scoring,
//! metabolism. Never a merge gate (C11).

// Four instruments answer on demand through their own tests and have no
// product caller; compiling them into the shipped library would ship an
// instrument, not a capability (eval-SPEC 8-3, 8-4, 8-5, 8-7).
#[cfg(test)]
mod ablation;
#[cfg(test)]
mod metabolism;
#[cfg(test)]
mod nesting;
mod probe;
#[cfg(test)]
mod score;
mod suite;

pub use probe::{Answers, Comparison, Probe, ProbeId, compare, handoff_probe};
pub use suite::{Half, Outcome, Report, Suite, Tally, Task};
