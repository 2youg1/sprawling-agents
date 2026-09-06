// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where the money went. Four independent cuts of
//! one authoritative total: by Run, by actor, by prefix segment, by
//! tool.
//!
//! The invariant that makes the report trustworthy (A20) is that each
//! cut sums to exactly the billed total — not approximately, not after
//! rounding. Percentages cannot promise that, so nothing here is a
//! percentage: shares are split by the largest-remainder method, which
//! distributes the floor divisions and then hands the leftover units
//! out one at a time, in a fixed order. What is billed is what is
//! attributed, to the microdollar.
//!
//! The total comes from `model_returned.billed_usd_micros` and nowhere
//! else. This module never prices a call — that authority is
//! `gateway::cost`, and a second one would be a second answer.

mod report;
mod split;

pub use report::{Attribution, AttributionReport};
