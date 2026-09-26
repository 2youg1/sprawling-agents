// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Deterministic city simulator, the second Main: thin executor, fixed
//! scripts, a counted clock, fault injection. There is no random source
//! here and no seed: a scenario is a script written out in a test, and
//! what makes it replay is that the script, the tick counter and the
//! single thread are all fixed. Dev-only workspace member; not in the
//! product graph.

// Four instruments answer on demand through their own tests and have no
// scenario caller; they compile only under test so the simulator's library
// carries scenarios, not instruments (citysim-SPEC.md 8-8).
#[cfg(test)]
mod ablation;
mod checker;
mod executor;
mod mem_ledger;
#[cfg(test)]
mod metabolism;
#[cfg(test)]
mod nesting;
mod red_team;
#[cfg(test)]
mod score;
mod script_model;
mod script_tools;
mod sieving;
mod suite;

pub use checker::check_chain;
pub use executor::{CancelPoint, Scenario, ScenarioReport, run_scenario, run_scenario_on};
pub use mem_ledger::MemLedger;
pub use red_team::{Arm, Case, Claim, Comparison, Plant, compare};
pub use script_model::{ScriptModel, concluding};
pub use script_tools::{ScriptTool, ScriptToolSet};
pub use sieving::SieveWorld;
pub use suite::{Half, Outcome, Report, Suite, Tally, Task};
