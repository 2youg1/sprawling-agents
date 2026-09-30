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

//! Attribution splits: dividing a bill by weight.

use std::collections::BTreeMap;

use kernel::UsdMicros;
use serde_json::Value;

use super::report::WINDOW_SLOT;
pub(crate) fn add(map: &mut BTreeMap<String, u64>, key: &str, amount: u64) {
    let slot = map.entry(key.to_owned()).or_insert(0);
    *slot = slot.saturating_add(amount);
}

pub(crate) fn quantify(map: &BTreeMap<String, u64>) -> Vec<(String, UsdMicros)> {
    map.iter()
        .map(|(k, v)| (k.clone(), UsdMicros::new(*v)))
        .collect()
}

/// The prefix shape as the ledger recorded it: four segment slots plus
/// the in-window history, weighted by bytes.
pub(crate) fn segment_weights(data: &serde_json::Map<String, Value>) -> Vec<(String, u64)> {
    let mut weights = Vec::new();
    if let Some(segments) = data.get("segments").and_then(Value::as_array) {
        for segment in segments {
            let slot = segment
                .get("slot")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_owned();
            let len = segment.get("len").and_then(Value::as_u64).unwrap_or(0);
            weights.push((slot, len));
        }
    }
    if let Some(window) = data.get("window_bytes").and_then(Value::as_u64) {
        weights.push((WINDOW_SLOT.to_owned(), window));
    }
    weights
}

/// Largest remainder: floor every share, then hand the leftover units
/// to the largest remainders, ties broken by bucket name. The result
/// sums to `total` exactly, for any weights, including all-zero ones.
pub(crate) fn split(total: u64, weights: &[(String, u64)], fallback: &str) -> Vec<(String, u64)> {
    let sum: u64 = weights.iter().map(|(_, w)| *w).fold(0, u64::saturating_add);
    if weights.is_empty() || sum == 0 {
        return vec![(fallback.to_owned(), total)];
    }
    let total_wide = u128::from(total);
    let sum_wide = u128::from(sum);
    let mut shares: Vec<(String, u64, u128)> = Vec::new();
    let mut assigned: u64 = 0;
    for (bucket, weight) in weights {
        let scaled = total_wide.saturating_mul(u128::from(*weight));
        let floor = scaled.checked_div(sum_wide).unwrap_or(0);
        let remainder = scaled.checked_rem(sum_wide).unwrap_or(0);
        let floor_u64 = u64::try_from(floor).unwrap_or(u64::MAX);
        assigned = assigned.saturating_add(floor_u64);
        shares.push((bucket.clone(), floor_u64, remainder));
    }
    // Order by remainder descending, then by name ascending, so the
    // leftover units land in the same place on every machine.
    let mut order: Vec<usize> = (0..shares.len()).collect();
    order.sort_by(|a, b| {
        let left = shares.get(*a);
        let right = shares.get(*b);
        match (left, right) {
            (Some(l), Some(r)) => r.2.cmp(&l.2).then_with(|| l.0.cmp(&r.0)),
            _ => std::cmp::Ordering::Equal,
        }
    });
    let mut leftover = total.saturating_sub(assigned);
    for index in order {
        if leftover == 0 {
            break;
        }
        if let Some(entry) = shares.get_mut(index) {
            entry.1 = entry.1.saturating_add(1);
            leftover = leftover.saturating_sub(1);
        }
    }
    // Buckets repeating a name (two segments in one slot) merge, so the
    // caller never sees the same key twice with different amounts.
    let mut merged: BTreeMap<String, u64> = BTreeMap::new();
    for (bucket, share, _) in shares {
        let slot = merged.entry(bucket).or_insert(0);
        *slot = slot.saturating_add(share);
    }
    merged.into_iter().collect()
}
