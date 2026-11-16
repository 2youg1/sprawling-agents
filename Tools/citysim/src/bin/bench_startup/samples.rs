// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::time::Duration;

pub use sprawling::monitor::spread::Share;
use sprawling::monitor::spread::{SUSPICIOUS_TIMES, Spread};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleKind {
    Plain,

    Suspicious,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Within,
    Outside,
}

const SECOND_TIER: Duration = Duration::from_millis(1);

pub struct Samples {
    spread: Spread,
    marks: Vec<SampleKind>,
}

impl Samples {

    #[must_use]
    pub fn of(head: Duration, tail: Vec<Duration>) -> Samples {
        let spread = Spread::of(head, tail.iter().copied());
        let cut = spread.p(Share::P50).saturating_mul(SUSPICIOUS_TIMES);
        let marks = std::iter::once(head)
            .chain(tail)
            .map(|time| {
                if time > cut {
                    SampleKind::Suspicious
                } else {
                    SampleKind::Plain
                }
            })
            .collect();
        Samples { spread, marks }
    }

    #[must_use]
    pub fn p(&self, share: Share) -> Duration {
        self.spread.p(share)
    }

    #[must_use]
    pub fn floor(&self) -> Duration {
        self.spread.floor()
    }

    #[must_use]
    pub fn peak(&self) -> Duration {
        self.spread.peak()
    }

    #[must_use]
    pub fn kind_at(&self, index: usize) -> SampleKind {
        self.marks.get(index).copied().unwrap_or(SampleKind::Plain)
    }

    #[must_use]
    pub fn suspicious(&self) -> usize {
        self.marks
            .iter()
            .filter(|mark| **mark == SampleKind::Suspicious)
            .count()
    }

    #[must_use]
    pub fn tier(&self) -> Tier {
        if self.spread.p(Share::P99) <= SECOND_TIER {
            Tier::Within
        } else {
            Tier::Outside
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::{SampleKind, Samples, Share, Tier};
    use std::time::Duration;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn the_tail_reads_the_nearest_rank_whatever_order_they_arrived_in() {
        let arrival: Vec<Duration> = (1..=200_u64).rev().map(ms).collect();
        let mut into = arrival.into_iter();
        let set = Samples::of(into.next().unwrap(), into.collect());
        assert_eq!(set.p(Share::P50), ms(100));
        assert_eq!(set.p(Share::P95), ms(190));
        assert_eq!(set.p(Share::P99), ms(198));
        assert_eq!(set.floor(), ms(1));
        assert_eq!(set.peak(), ms(200));
    }

    #[test]
    fn a_sample_three_times_the_middle_is_marked_and_kept() {
        let set = Samples::of(ms(10), vec![ms(10), ms(10), ms(30), ms(40)]);
        assert_eq!(set.kind_at(3), SampleKind::Plain);
        assert_eq!(set.kind_at(4), SampleKind::Suspicious);
        assert_eq!(set.suspicious(), 1);
        assert_eq!(set.peak(), ms(40));
    }

    #[test]
    fn the_second_tier_is_where_the_tail_still_fits_one_millisecond() {
        assert_eq!(Samples::of(ms(1), vec![ms(1)]).tier(), Tier::Within);
        assert_eq!(Samples::of(ms(1), vec![ms(2)]).tier(), Tier::Outside);
    }
}
