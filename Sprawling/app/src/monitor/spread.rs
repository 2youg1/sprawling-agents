// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one reading of a set of durations: its floor, its nearest-rank
//! shares, its peak, and how many samples sit far above the middle
//! (`crates/sprawling/spec/Main.lean` §8-129-2).
//!
//! Every percentile the repository prints - `sprawling gauge`'s spread
//! line, citysim's `bench` and `bench_startup` readings - is read here,
//! so the same samples give the same p50 wherever they are printed.

use std::time::Duration;

/// How far above the middle a sample is counted suspicious: a sample
/// above this many times p50 looks like interference (an on-access
/// scan, a scheduler hiccup) rather than the work itself. Counted,
/// never removed.
pub const SUSPICIOUS_TIMES: u32 = 3;

/// Which order statistic is asked for. Three named shares rather than a
/// percent parameter: every reading prints these three and no other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Share {
    P50,
    P95,
    P99,
}

/// One set of at least one duration, read once at construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spread {
    samples: u64,
    floor: Duration,
    p50: Duration,
    p95: Duration,
    p99: Duration,
    peak: Duration,
    suspicious: u64,
}

impl Spread {
    /// The spread of `head` and every duration in `tail`, in any order.
    /// The head is a parameter rather than a check, so a set with no
    /// sample cannot be spelled.
    #[must_use]
    pub fn of(head: Duration, tail: impl IntoIterator<Item = Duration>) -> Spread {
        let mut sorted: Vec<Duration> = std::iter::once(head).chain(tail).collect();
        sorted.sort_unstable();
        let pick = |share| nearest_rank(&sorted, share).unwrap_or(head);
        let p50 = pick(Share::P50);
        let cut = p50.saturating_mul(SUSPICIOUS_TIMES);
        Spread {
            samples: u64::try_from(sorted.len()).unwrap_or(u64::MAX),
            floor: sorted.first().copied().unwrap_or(head),
            p50,
            p95: pick(Share::P95),
            p99: pick(Share::P99),
            peak: sorted.last().copied().unwrap_or(head),
            suspicious: u64::try_from(sorted.iter().filter(|time| **time > cut).count())
                .unwrap_or(u64::MAX),
        }
    }

    /// The nearest-rank order statistic: rank `ceil(n * p / 100)`,
    /// counted from one.
    #[must_use]
    pub fn p(&self, share: Share) -> Duration {
        match share {
            Share::P50 => self.p50,
            Share::P95 => self.p95,
            Share::P99 => self.p99,
        }
    }

    /// The smallest sample: what the work costs while the machine is
    /// quiet.
    #[must_use]
    pub fn floor(&self) -> Duration {
        self.floor
    }

    /// The largest sample.
    #[must_use]
    pub fn peak(&self) -> Duration {
        self.peak
    }

    /// How many samples the set holds.
    #[must_use]
    pub fn samples(&self) -> u64 {
        self.samples
    }

    /// How many samples sit above [`SUSPICIOUS_TIMES`] times p50.
    #[must_use]
    pub fn suspicious(&self) -> u64 {
        self.suspicious
    }
}

/// A duration as the whole number of microseconds every reading line
/// and register field carries (`crates/sprawling/spec/Main.lean`
/// §8-129-2, *单位*); a span too long for `u64` reads as `u64::MAX`.
#[must_use]
pub fn micros(span: Duration) -> u64 {
    u64::try_from(span.as_micros()).unwrap_or(u64::MAX)
}

/// A duration as a whole number of nanoseconds, for an in-process
/// reading below one microsecond and for the display rule of
/// `monitor::top::Unit::Nanos`.
#[must_use]
pub fn nanos(span: Duration) -> u64 {
    u64::try_from(span.as_nanos()).unwrap_or(u64::MAX)
}

/// The sample of rank `ceil(n * p / 100)` in an ascending set, which
/// over whole numbers sits at index `(n * p - 1) / 100`. `None` only for
/// an empty set, which [`Spread::of`] cannot build.
fn nearest_rank(sorted: &[Duration], share: Share) -> Option<Duration> {
    let percent: usize = match share {
        Share::P50 => 50,
        Share::P95 => 95,
        Share::P99 => 99,
    };
    let index = sorted.len().saturating_mul(percent).saturating_sub(1) / 100;
    sorted.get(index).copied()
}

#[cfg(test)]
#[path = "spread/tests.rs"]
mod tests;
