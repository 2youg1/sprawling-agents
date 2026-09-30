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

/// Which order statistic is asked for.
///
/// Three named shares rather than a percent parameter: the report asks
/// these three and no other, and a parameter would accept a question
/// nobody answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Share {
    P50,
    P95,
    P99,
}

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

/// How far above the middle a sample is flagged suspicious, in halves
/// kept whole: three times p50 (citysim-SPEC.md section 8-5).
const SUSPICIOUS_TIMES: u32 = 3;

/// One action's sample set: at least one duration, in the order it was
/// recorded.
///
/// Everything a report reads is derived at the one construction point,
/// so a query is a field read and no reader can disagree with another
/// about what the middle was. The order is arrival order on purpose: a
/// suspicious stretch is a question about *when* in the run something
/// happened, so the set keeps the timeline and sorts only for an order
/// statistic.
pub struct Samples {
    p50: Duration,
    p95: Duration,
    p99: Duration,
    floor: Duration,
    peak: Duration,
    marks: Vec<SampleKind>,
}

impl Samples {
    /// A sample set of `head` and every duration in `tail`.
    ///
    /// The head is a parameter rather than a check, so a set with no
    /// sample cannot be spelled.
    #[must_use]
    pub fn of(head: Duration, tail: Vec<Duration>) -> Samples {
        let mut arrival = Vec::with_capacity(tail.len().saturating_add(1));
        arrival.push(head);
        arrival.extend(tail);
        let mut sorted = arrival.clone();
        sorted.sort();
        let middle = at(&sorted, index_of(sorted.len(), Share::P50));
        let cut = middle.saturating_mul(SUSPICIOUS_TIMES);
        let marks = arrival
            .iter()
            .map(|time| {
                if *time > cut {
                    SampleKind::Suspicious
                } else {
                    SampleKind::Plain
                }
            })
            .collect();
        let floor = arrival.iter().copied().fold(head, Duration::min);
        let peak = arrival.iter().copied().fold(head, Duration::max);
        Samples {
            p50: middle,
            p95: at(&sorted, index_of(sorted.len(), Share::P95)),
            p99: at(&sorted, index_of(sorted.len(), Share::P99)),
            floor,
            peak,
            marks,
        }
    }

    /// The nearest-rank order statistic: rank `ceil(share * n)`, one-based.
    #[must_use]
    pub fn p(&self, share: Share) -> Duration {
        match share {
            Share::P50 => self.p50,
            Share::P95 => self.p95,
            Share::P99 => self.p99,
        }
    }

    /// The smallest sample: the limit reading the report records.
    #[must_use]
    pub fn floor(&self) -> Duration {
        self.floor
    }

    /// The largest sample.
    #[must_use]
    pub fn peak(&self) -> Duration {
        self.peak
    }

    /// What one sample looks like, by its place in the run.
    ///
    /// A place the run never recorded reads `Plain`: nothing was measured
    /// there, which is the same convention `at` names for a pick.
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
        if self.p99 <= SECOND_TIER {
            Tier::Within
        } else {
            Tier::Outside
        }
    }
}

/// Where the order statistic for `share` sits in an `n`-sample set,
/// zero-based: rank `ceil(share * n)` minus one, which over whole
/// numbers is `(n * numerator - 1) / 100`.
///
/// Every step saturates: the arithmetic is on a sample count, not on a
/// value anything depends on wrapping.
fn index_of(n: usize, share: Share) -> usize {
    let numerator = match share {
        Share::P50 => 50,
        Share::P95 => 95,
        Share::P99 => 99,
    };
    n.saturating_mul(numerator).saturating_sub(1) / 100
}

/// The value at `index` of an ascending set.
///
/// The pick always lands: `index_of` returns `ceil(n * q / 100) - 1`,
/// which is below `n` for every share here and `n` is at least one by
/// construction. A fold rather than `Vec::get`, because the fallback an
/// index needs would be a reading nobody measured.
fn at(sorted: &[Duration], index: usize) -> Duration {
    debug_assert!(
        index < sorted.len(),
        "rank arithmetic kept outside the set: index {index} of {}",
        sorted.len()
    );
    sorted.iter().enumerate().fold(
        Duration::ZERO,
        |picked, (i, time)| {
            if i == index { *time } else { picked }
        },
    )
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
