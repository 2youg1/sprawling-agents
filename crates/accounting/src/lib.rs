// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The accounting thread's ports: what the city's one writer reaches
//! through when it needs something from outside itself.
//!
//! A writer that builds its own dependencies can be driven against only
//! one of each, which is why a scripted scenario could reproduce a run
//! and not a dispatch (ARCHITECTURE.md section 11). Each port here has a
//! production adapter in `bin::assembly` and a second one outside it.
//!
//! Beside the ports live the worker's own values that reach nothing but
//! the kernel, the city's files and the collaboration vocabulary: what a
//! desk's effects come to (`effect`) and every building's plan folded
//! from the records (`plan_view`).

mod clock;
mod connectors;
pub mod effect;
mod machine;
mod models;
pub mod plan_view;

pub use clock::Clock;
pub use connectors::Connectors;
pub use machine::{Machine, Recipe, Runnable};
pub use models::ModelFactory;
