// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Citysim scenario suite as one integration binary.
//!
//! Each former `tests/*.rs` file linked the same citysim/accounting/runtime
//! graph once; ten copies paid ten link steps in the CI test archive. One
//! binary keeps the same tests and nextest filters (`package(=citysim)`).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

mod collaboration;
mod compaction;
mod evaluation;
mod freeze_leak;
mod hazards;
mod interventions;
mod ledger_conformance;
mod proposal_baseline;
mod scenario;
mod sieve;
