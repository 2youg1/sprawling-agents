// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which arms the throughput bench reads, and the line each wait prints.

use super::P999_FLOOR;

/// The arms an environment variable names, as a comma list of their
/// names, or every arm when it is unset.
pub(super) fn chosen<T: Copy>(var: &str, all: &[T], name: impl Fn(&T) -> String) -> Vec<T> {
    match std::env::var(var) {
        Ok(list) => all
            .iter()
            .filter(|arm| list.split(',').any(|item| item.trim() == name(arm)))
            .copied()
            .collect(),
        Err(_) => all.to_vec(),
    }
}

/// One wait's line: n, p50, p99, and p999 when there are `P999_FLOOR`
/// samples, else the mark that there are too few; the max always.
pub(super) fn wait_line(head: &str, wait: &str, mut samples: Vec<u64>) -> String {
    samples.sort_unstable();
    let n = samples.len();
    let at = |permille: usize| {
        let rank = (n * permille).div_ceil(1000).max(1);
        samples.get(rank - 1).copied().unwrap_or(0)
    };
    let tail = if n >= P999_FLOOR {
        format!("p999_us={}", at(999))
    } else {
        "p999=insufficient".to_owned()
    };
    format!(
        "throughput_wait {head} wait={wait} n={n} p50_us={} p99_us={} {tail} max_us={}",
        at(500),
        at(990),
        samples.last().copied().unwrap_or(0)
    )
}
