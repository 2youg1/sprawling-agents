// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::time::Duration;

pub const SUSPICIOUS_TIMES: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Share {
    P50,
    P95,
    P99,
}

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

    #[must_use]
    pub fn p(&self, share: Share) -> Duration {
        match share {
            Share::P50 => self.p50,
            Share::P95 => self.p95,
            Share::P99 => self.p99,
        }
    }

    #[must_use]
    pub fn floor(&self) -> Duration {
        self.floor
    }

    #[must_use]
    pub fn peak(&self) -> Duration {
        self.peak
    }

    #[must_use]
    pub fn samples(&self) -> u64 {
        self.samples
    }

    #[must_use]
    pub fn suspicious(&self) -> u64 {
        self.suspicious
    }
}

#[must_use]
pub fn micros(span: Duration) -> u64 {
    u64::try_from(span.as_micros()).unwrap_or(u64::MAX)
}

#[must_use]
pub fn nanos(span: Duration) -> u64 {
    u64::try_from(span.as_nanos()).unwrap_or(u64::MAX)
}

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
