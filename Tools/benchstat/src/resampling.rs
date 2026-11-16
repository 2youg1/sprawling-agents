// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Deterministic dependent-data resampling and BCa intervals.

use sha2::{Digest, Sha256};

use crate::error::BenchError;
use crate::estimators::{count, mean};

/// Shared confidence and draw count for one resampled statistic.
#[derive(Debug, Clone, Copy)]
pub struct ResamplingContract {
    /// Number of bootstrap or permutation draws.
    pub resamples: u32,
    /// Two-sided interval confidence.
    pub confidence: f64,
}

/// One interval and its transparent fallback diagnostics.
#[derive(Debug, Clone)]
pub struct Interval {
    /// BCa endpoints, absent when its assumptions fail.
    pub bca: Option<(f64, f64)>,
    /// Percentile sensitivity endpoints, always available for a non-empty sample.
    pub percentile: (f64, f64),
    /// `bca`, `percentile_fallback` or `iid_sensitivity`.
    pub status: String,
    /// Why BCa is unavailable.
    pub reason: Option<String>,
    /// Selected moving-block length per independent segment.
    pub block_lengths: Vec<usize>,
    /// Number of finite bootstrap replicates used.
    pub usable_replicates: u32,
}

mod block;
pub use block::BlockBootstrap;

/// A reproducible SHA-256 counter stream.
#[derive(Debug)]
pub struct SeedStream {
    material: Vec<u8>,
    counter: u64,
    digest: [u8; 32],
    word: usize,
}

impl SeedStream {
    /// Derive a stream from length-prefixed UTF-8 fields.
    pub fn new(fields: &[&str]) -> Result<Self, BenchError> {
        let mut material = Vec::new();
        for field in fields {
            let length = u64::try_from(field.len())
                .map_err(|_| numeric("seed derivation", "field length exceeds u64"))?;
            material.extend(length.to_be_bytes());
            material.extend(field.as_bytes());
        }
        Ok(Self {
            material,
            counter: 0,
            digest: [0; 32],
            word: 4,
        })
    }

    /// Uniform integer below `bound`, with rejection rather than modulo bias.
    pub fn index(&mut self, bound: usize) -> Result<usize, BenchError> {
        if bound == 0 {
            return Err(numeric("random index", "zero bound"));
        }
        let bound64 =
            u64::try_from(bound).map_err(|_| numeric("random index", "bound exceeds u64"))?;
        let threshold = bound64
            .wrapping_neg()
            .checked_rem(bound64)
            .ok_or_else(|| numeric("random index", "zero rejection bound"))?;
        loop {
            let value = self.next_u64()?;
            if value >= threshold {
                let selected = value
                    .checked_rem(bound64)
                    .ok_or_else(|| numeric("random index", "zero modulo bound"))?;
                return usize::try_from(selected)
                    .map_err(|_| numeric("random index", "selected index exceeds usize"));
            }
        }
    }

    fn next_u64(&mut self) -> Result<u64, BenchError> {
        if self.word >= 4 {
            self.counter = self
                .counter
                .checked_add(1)
                .ok_or_else(|| numeric("seed stream", "counter exhausted"))?;
            let mut hasher = Sha256::new();
            hasher.update(&self.material);
            hasher.update(self.counter.to_be_bytes());
            self.digest = hasher.finalize().into();
            self.word = 0;
        }
        let start = self
            .word
            .checked_mul(8)
            .ok_or_else(|| numeric("seed stream", "word offset overflow"))?;
        let end = start
            .checked_add(8)
            .ok_or_else(|| numeric("seed stream", "word end overflow"))?;
        let bytes: [u8; 8] = self
            .digest
            .get(start..end)
            .ok_or_else(|| numeric("seed stream", "digest exhausted"))?
            .try_into()
            .map_err(|_| numeric("seed stream", "wrong digest word size"))?;
        self.word = self
            .word
            .checked_add(1)
            .ok_or_else(|| numeric("seed stream", "word counter overflow"))?;
        Ok(u64::from_be_bytes(bytes))
    }
}

/// Politis–White choice and its exported diagnostics.
#[derive(Debug, Clone, Copy)]
pub struct BlockChoice {
    /// Moving-block length.
    pub length: usize,
    /// Flat-top bandwidth M.
    pub bandwidth: usize,
    /// Estimated G.
    pub g: f64,
    /// Corrected circular-bootstrap variance constant.
    pub d_cb: f64,
}

/// Select a moving-block length with the 2009-corrected Politis–White procedure.
pub fn politis_white(values: &[f64]) -> Result<BlockChoice, BenchError> {
    if values.len() < 4 {
        return Ok(BlockChoice {
            length: 1,
            bandwidth: 0,
            g: 0.0,
            d_cb: 0.0,
        });
    }
    let centre = mean(values).ok_or_else(|| numeric("block selection", "empty series"))?;
    let n = count(values.len())?;
    let max_lag = values.len().checked_div(4).unwrap_or(1).max(1);
    let mut covariance = Vec::new();
    for lag in 0..=max_lag {
        let mut sum = 0.0;
        for index in 0..values.len() {
            let paired = index.checked_add(lag);
            if let Some(other) = paired.and_then(|position| values.get(position)) {
                let current = values
                    .get(index)
                    .ok_or_else(|| numeric("block selection", "index out of bounds"))?;
                sum += (current - centre) * (other - centre);
            }
        }
        covariance.push(sum / n);
    }
    let r0 = covariance.first().copied().unwrap_or(0.0);
    if r0 == 0.0 {
        return Ok(BlockChoice {
            length: 1,
            bandwidth: 0,
            g: 0.0,
            d_cb: 0.0,
        });
    }
    let threshold = 2.0 * (n.log10() / n).sqrt();
    let mut k_n = 5_usize;
    while count(k_n)?.powi(2) < n.log10() {
        k_n = k_n
            .checked_add(1)
            .ok_or_else(|| numeric("block selection", "correlogram window overflow"))?;
    }
    let mut m = max_lag;
    for candidate in 1..=max_lag {
        let mut negligible = true;
        for offset in 1..=k_n {
            let lag = candidate.checked_add(offset);
            if let Some(r) = lag.and_then(|index| covariance.get(index))
                && (r / r0).abs() >= threshold
            {
                negligible = false;
                break;
            }
        }
        if negligible {
            m = candidate;
            break;
        }
    }
    let bandwidth = m.saturating_mul(2).min(max_lag);
    let mut g = 0.0;
    let mut spectral = r0;
    for lag in 1..=bandwidth {
        let r = covariance.get(lag).copied().unwrap_or(0.0);
        let weight = flat_top(count(lag)? / count(bandwidth)?);
        g += 2.0 * weight * count(lag)? * r;
        spectral += 2.0 * weight * r;
    }
    let d_cb = 4.0 * spectral.powi(2) / 3.0;
    let length = if g == 0.0 || d_cb <= 0.0 || !d_cb.is_finite() {
        1
    } else {
        let raw = (2.0 * g.powi(2) / d_cb).powf(1.0 / 3.0) * n.powf(1.0 / 3.0);
        integer_nearest(raw, values.len())?
    };
    Ok(BlockChoice {
        length,
        bandwidth,
        g,
        d_cb,
    })
}

/// Inverse empirical CDF for an arbitrary probability, including BCa-adjusted tails.
pub fn order_quantile(sorted: &[f64], probability: f64) -> Result<f64, BenchError> {
    if sorted.is_empty() || !(0.0..=1.0).contains(&probability) {
        return Err(numeric(
            "order quantile",
            "empty sample or probability outside [0,1]",
        ));
    }
    let n = count(sorted.len())?;
    let mut low = 0_usize;
    let mut high = sorted.len();
    while low < high {
        let middle = low
            .checked_add(
                high.checked_sub(low)
                    .ok_or_else(|| numeric("order quantile", "index underflow"))?
                    / 2,
            )
            .ok_or_else(|| numeric("order quantile", "index overflow"))?;
        let rank = count(
            middle
                .checked_add(1)
                .ok_or_else(|| numeric("order quantile", "rank overflow"))?,
        )?;
        if rank / n >= probability {
            high = middle;
        } else {
            low = middle
                .checked_add(1)
                .ok_or_else(|| numeric("order quantile", "index overflow"))?;
        }
    }
    sorted
        .get(low.min(sorted.len().saturating_sub(1)))
        .copied()
        .ok_or_else(|| numeric("order quantile", "rank out of bounds"))
}

fn integer_nearest(value: f64, ceiling: usize) -> Result<usize, BenchError> {
    let mut answer = 1_usize;
    for candidate in 1..=ceiling {
        let distance = (count(candidate)? - value).abs();
        let incumbent = (count(answer)? - value).abs();
        if distance <= incumbent {
            answer = candidate;
        }
    }
    Ok(answer)
}

fn flat_top(distance: f64) -> f64 {
    if distance <= 0.5 {
        1.0
    } else if distance <= 1.0 {
        2.0 * (1.0 - distance)
    } else {
        0.0
    }
}

fn numeric(action: &str, subject: &str) -> BenchError {
    BenchError::numeric(
        action,
        subject.to_owned(),
        "inspect the dependent observations and sizing parameters",
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests;
