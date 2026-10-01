// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The city's one writer (`worker`), and the ports it reaches through
//! when it needs something from outside itself.
//!
//! A writer that builds its own dependencies can be driven against only
//! one of each, which is why a scripted scenario could reproduce a run
//! and not a dispatch (ARCHITECTURE.md section 11). The worker receives
//! this machine as one `worker::hands::Hands` value; the production one
//! is made in `bin::assembly::production`, and every port has a second
//! implementation outside it (accounting-SPEC.md 8-11).
//!
//! Beside the ports live the worker's own values that reach nothing but
//! the kernel, the city's files and the collaboration vocabulary: what a
//! desk's effects come to (`effect`) and every building's plan folded
//! from the records (`plan_view`).
//!
//! The read side lives here too: the fold every page is answered from
//! (`views`), and every run folded into one line with its parent
//! pointers (`lineage`), which the binary's `view` command and the
//! views both read (accounting-SPEC.md 8-10), and a stretch of that
//! history exported as a playback bundle anyone can recompute
//! (`playback`, accounting-SPEC.md 8-12), and a commit traced back to
//! the calls its run made before it (`trace`, accounting-SPEC.md 8-16).

mod clock;
mod connectors;
pub mod effect;
pub mod held_vault;
pub mod home;
pub mod lineage;
mod machine;
mod models;
pub mod person;
pub mod plan_view;
pub mod playback;
pub mod toolkit_broker;
pub mod trace;
pub mod views;
pub mod worker;

pub use clock::Clock;
pub use connectors::{Connectors, Reached};
pub use machine::{Machine, Recipe, Runnable};
pub use models::ModelFactory;
