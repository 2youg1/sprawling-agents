// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One assertion suite for every model implementation (V3).

use super::{Model, ModelRequest};

/// The universal model contract is thin on purpose: two consecutive
/// calls must both *return* — no panic, and no poisoned state after an
/// Err. Result shapes are already the types' business; determinism is
/// not asserted (real providers are not deterministic — scripted
/// adapters prove theirs in citysim).
#[allow(
    clippy::panic,
    reason = "conformance suites assert by panicking; they are dev-only by feature"
)]
#[cfg(feature = "conformance")]
pub fn assert_model_conformance<M: Model>(model: &mut M, benign: &ModelRequest) {
    for round in 0..2u8 {
        match model.call(benign) {
            Ok(ret) => assert!(
                ret.calls.len() != usize::MAX,
                "round {round}: adapter returned a wave"
            ),
            Err(err) => assert!(
                !err.code().as_str().is_empty(),
                "round {round}: adapter returned a typed error"
            ),
        }
    }
}
