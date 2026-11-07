// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Reproducible benchmark analysis (tools/benchstat/Spec.lean).

pub mod analysis;
pub mod error;
pub mod estimators;
pub mod manifest;
pub mod observation;
pub mod report;
pub mod resampling;
mod round;
pub mod shape;

pub use error::BenchError;
pub use round::{Computed, export, export_write, report, verify};
