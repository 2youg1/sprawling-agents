// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Every share is the nearest rank a person counts by hand.

use std::time::Duration;

use super::Spread;

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

/// `1..=n` milliseconds, largest first: a share read off arrival order
/// would pass on a sorted set and be wrong on a real run.
fn descending(n: u64) -> Spread {
    let mut arrival = (1..=n).rev().map(ms);
    let head = arrival.next().unwrap_or_default();
    Spread::of(head, arrival)
}

#[test]
fn every_share_is_the_hand_counted_nearest_rank() {
    let expected = |samples, p50, p95, p99| Spread {
        samples,
        floor: ms(1),
        p50: ms(p50),
        p95: ms(p95),
        p99: ms(p99),
        peak: ms(samples),
        suspicious: 0,
    };
    assert_eq!(
        [1, 2, 100, 200].map(descending),
        [
            expected(1, 1, 1, 1),
            expected(2, 1, 2, 2),
            expected(100, 50, 95, 99),
            expected(200, 100, 190, 198),
        ]
    );
}

/// The count is a suspicion, not a deletion: the sample still sets the
/// peak.
#[test]
fn a_sample_above_three_times_the_middle_is_counted_and_kept() {
    assert_eq!(
        Spread::of(ms(10), [ms(10), ms(10), ms(30), ms(40)]),
        Spread {
            samples: 5,
            floor: ms(10),
            p50: ms(10),
            p95: ms(40),
            p99: ms(40),
            peak: ms(40),
            suspicious: 1,
        }
    );
}
