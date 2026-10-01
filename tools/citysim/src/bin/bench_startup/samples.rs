// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One action's timed samples, and what they answer (citysim-SPEC.md
//! section 8-5).
//!
//! Shape: decision. Every reading arrives as a `Duration` a caller
//! stamped elsewhere, so this module holds no clock and no I/O and can
//! be tested against hand-written numbers. Three answers, one per part
//! of the three-piece delivery: [`Tier`] says whether the tail fits the
//! second tier, `floor` and `peak` are the extreme readings, and
//! [`SampleKind`] marks the samples an outside interferer (Defender's
//! real-time scan, a scheduler hiccup) is the likeliest explanation of.

use std::time::Duration;

/// Which order statistic is asked for: the product's own, so a share
/// here and in a `bench` or `gauge` reading names the same sample.
pub use sprawling::monitor::spread::Share;
use sprawling::monitor::spread::{SUSPICIOUS_TIMES, Spread};

/// Whether one sample looks like the action or like interference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleKind {
    Plain,
    /// Above three times the middle sample: flagged, never removed.
    Suspicious,
}

/// Whether the measured tail fits the second tier, p99 within 1 ms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Within,
    Outside,
}

/// The second tier's line: p99 within one millisecond (citysim-SPEC.md
/// section 8-5).
const SECOND_TIER: Duration = Duration::from_millis(1);

/// One action's sample set: at least one duration, in the order it was
/// recorded.
///
/// The shares, floor and peak are the product's one reading of a set,
/// `Spread` (sprawling-SPEC.md 8-129-2); what this set adds is the
/// arrival order. A suspicious stretch is a question about *when* in the
/// run something happened, so each sample keeps its mark in the order it
/// arrived.
pub struct Samples {
    spread: Spread,
    marks: Vec<SampleKind>,
}

impl Samples {
    /// A sample set of `head` and every duration in `tail`.
    ///
    /// The head is a parameter rather than a check, so a set with no
    /// sample cannot be spelled.
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

    /// The nearest-rank order statistic: rank `ceil(share * n)`, one-based.
    #[must_use]
    pub fn p(&self, share: Share) -> Duration {
        self.spread.p(share)
    }

    /// The smallest sample: the limit reading the report records.
    #[must_use]
    pub fn floor(&self) -> Duration {
        self.spread.floor()
    }

    /// The largest sample.
    #[must_use]
    pub fn peak(&self) -> Duration {
        self.spread.peak()
    }

    /// What one sample looks like, by its place in the run.
    ///
    /// A place the run never recorded reads `Plain`: nothing was measured
    /// there.
    #[must_use]
    pub fn kind_at(&self, index: usize) -> SampleKind {
        self.marks.get(index).copied().unwrap_or(SampleKind::Plain)
    }

    /// How many samples are marked suspicious.
    #[must_use]
    pub fn suspicious(&self) -> usize {
        self.marks
            .iter()
            .filter(|mark| **mark == SampleKind::Suspicious)
            .count()
    }

    /// Whether the tail fits the second tier.
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

    /// Two hundred samples in arrival order, largest first: a percentile
    /// read off arrival order would pass on a sorted fixture and be wrong
    /// on a real run.
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

    /// The flag is a suspicion, not a deletion: the sample is marked and
    /// still counted everywhere.
    #[test]
    fn a_sample_three_times_the_middle_is_marked_and_kept() {
        let set = Samples::of(ms(10), vec![ms(10), ms(10), ms(30), ms(40)]);
        assert_eq!(set.kind_at(3), SampleKind::Plain);
        assert_eq!(set.kind_at(4), SampleKind::Suspicious);
        assert_eq!(set.suspicious(), 1);
        assert_eq!(set.peak(), ms(40));
    }

    /// The second tier is p99 within 1 ms, so exactly 1 ms is inside it.
    #[test]
    fn the_second_tier_is_where_the_tail_still_fits_one_millisecond() {
        assert_eq!(Samples::of(ms(1), vec![ms(1)]).tier(), Tier::Within);
        assert_eq!(Samples::of(ms(1), vec![ms(2)]).tier(), Tier::Outside);
    }
}
