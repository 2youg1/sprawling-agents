// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Reproducible benchmark analysis. The public entry points perform the same
//! computation for generation and verification (tools/benchstat/Spec.lean §8).

use std::path::Path;

pub mod error;

pub use error::BenchError;

/// The two authoritative texts computed from one round.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Computed {
    /// Canonical exports, including the final LF.
    pub exports: String,
    /// The generated Markdown section, without its surrounding markers.
    pub markdown: String,
}

/// Validate and analyze one round without writing it.
///
/// # Errors
/// Returns a stable input or numerical error when the round cannot be analyzed.
pub fn export(round: &Path) -> Result<Computed, BenchError> {
    Err(BenchError::numeric(
        "analyze benchmark round",
        round.display().to_string(),
        "use the implemented estimator pipeline",
    ))
}

/// Recompute one round without changing it.
///
/// # Errors
/// Returns the same analysis errors as [`export`].
pub fn verify(round: &Path) -> Result<Computed, BenchError> {
    export(round)
}
