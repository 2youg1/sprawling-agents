// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Deterministic dependent-data resampling and BCa intervals.

use sha2::{Digest, Sha256};

use crate::error::BenchError;
use crate::estimators::{
    Quantile, count, mean, nearest_rank, normal_cdf, normal_quantile, pettitt,
};

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

/// Ordered observations partitioned at round/cohort boundaries.
#[derive(Debug, Clone)]
pub struct BlockBootstrap {
    segments: Vec<Vec<f64>>,
    lengths: Vec<usize>,
    ordered: bool,
}

impl BlockBootstrap {
    /// Select Politis–White lengths independently inside each segment.
    ///
    /// # Errors
    /// Returns a numerical error for an empty series or non-finite calculation.
    pub fn new(segments: Vec<Vec<f64>>, ordered: bool) -> Result<Self, BenchError> {
        if segments.is_empty()
            || segments
                .iter()
                .any(|segment| segment.iter().any(|v| !v.is_finite()))
        {
            return Err(numeric(
                "construct block bootstrap",
                "empty segment or non-finite observation",
            ));
        }
        let lengths = if ordered {
            segments
                .iter()
                .map(|segment| Ok(politis_white(segment)?.length))
                .collect::<Result<Vec<_>, BenchError>>()?
        } else {
            vec![1; segments.len()]
        };
        Ok(Self {
            segments,
            lengths,
            ordered,
        })
    }

    /// Selected lengths in segment order.
    pub fn lengths(&self) -> &[usize] {
        &self.lengths
    }

    /// Compute one quantile's dependence-preserving BCa and percentile sensitivity.
    ///
    /// # Errors
    /// Returns only setup/numerical errors; ordinary BCa degeneracy is an explicit fallback.
    pub fn quantile_interval(
        &self,
        original: &[f64],
        quantile: Quantile,
        contract: ResamplingContract,
        stream: &mut SeedStream,
    ) -> Result<Interval, BenchError> {
        let ResamplingContract {
            resamples,
            confidence,
        } = contract;
        let mut sorted = original.to_vec();
        sorted.sort_by(f64::total_cmp);
        let estimate = nearest_rank(&sorted, quantile)
            .ok_or_else(|| numeric("quantile interval", "empty original sample"))?;
        let mut replicates = Vec::new();
        for _ in 0..resamples {
            let sample = self.draw(stream)?;
            let mut sample = sample;
            sample.sort_by(f64::total_cmp);
            if let Some(value) = nearest_rank(&sample, quantile) {
                replicates.push(value);
            }
        }
        replicates.sort_by(f64::total_cmp);
        if replicates.is_empty() {
            return Err(numeric("quantile bootstrap", "no usable replicates"));
        }
        let alpha = (1.0 - confidence) / 2.0;
        let percentile = (
            order_quantile(&replicates, alpha)?,
            order_quantile(&replicates, 1.0 - alpha)?,
        );
        let usable_replicates = u32::try_from(replicates.len())
            .map_err(|_| numeric("quantile bootstrap", "replicate count exceeds u32"))?;
        let fallback = |reason: String| Interval {
            bca: None,
            percentile,
            status: if self.ordered {
                "percentile_fallback".to_owned()
            } else {
                "iid_sensitivity".to_owned()
            },
            reason: Some(reason),
            block_lengths: self.lengths.clone(),
            usable_replicates,
        };
        let bca = match self.bca(&replicates, estimate, quantile, confidence) {
            Ok(endpoints) => endpoints,
            Err(reason) => return Ok(fallback(reason)),
        };
        Ok(Interval {
            bca: Some(bca),
            percentile,
            status: if self.ordered {
                "bca".to_owned()
            } else {
                "iid_sensitivity".to_owned()
            },
            reason: None,
            block_lengths: self.lengths.clone(),
            usable_replicates,
        })
    }

    /// Permutation p-value that shuffles only inside independent segments.
    ///
    /// # Errors
    /// Returns an error for an empty permutation distribution.
    pub fn pettitt_p(
        &self,
        original: &[f64],
        resamples: u32,
        stream: &mut SeedStream,
    ) -> Result<Option<f64>, BenchError> {
        let Some((observed, _)) = pettitt(original) else {
            return Ok(None);
        };
        let mut at_least = 0_u32;
        for _ in 0..resamples {
            let sample = self.permutation(stream)?;
            if pettitt(&sample).is_some_and(|(statistic, _)| statistic >= observed) {
                at_least = at_least
                    .checked_add(1)
                    .ok_or_else(|| numeric("Pettitt permutation", "draw count overflow"))?;
            }
        }
        let numerator = count(
            usize::try_from(at_least)
                .map_err(|_| numeric("Pettitt permutation", "count conversion"))?,
        )?;
        let denominator = count(
            usize::try_from(resamples)
                .map_err(|_| numeric("Pettitt permutation", "count conversion"))?,
        )?;
        Ok(Some((1.0 + numerator) / (1.0 + denominator)))
    }

    pub(crate) fn draw(&self, stream: &mut SeedStream) -> Result<Vec<f64>, BenchError> {
        let mut output = Vec::new();
        for (segment, length) in self.segments.iter().zip(&self.lengths) {
            if segment.is_empty() {
                continue;
            }
            if !self.ordered || *length <= 1 {
                for _ in 0..segment.len() {
                    let index = stream.index(segment.len())?;
                    output.push(
                        *segment
                            .get(index)
                            .ok_or_else(|| numeric("bootstrap draw", "index out of bounds"))?,
                    );
                }
                continue;
            }
            let starts = segment
                .len()
                .checked_sub(*length)
                .and_then(|value| value.checked_add(1))
                .ok_or_else(|| numeric("bootstrap draw", "block exceeds segment"))?;
            let mut produced = 0_usize;
            while produced < segment.len() {
                let start = stream.index(starts)?;
                for offset in 0..*length {
                    let index = start
                        .checked_add(offset)
                        .ok_or_else(|| numeric("bootstrap draw", "block index overflow"))?;
                    output.push(
                        *segment
                            .get(index)
                            .ok_or_else(|| numeric("bootstrap draw", "block out of bounds"))?,
                    );
                    produced = produced
                        .checked_add(1)
                        .ok_or_else(|| numeric("bootstrap draw", "sample length overflow"))?;
                    if produced == segment.len() {
                        break;
                    }
                }
            }
        }
        Ok(output)
    }

    fn permutation(&self, stream: &mut SeedStream) -> Result<Vec<f64>, BenchError> {
        let mut output = Vec::new();
        for segment in &self.segments {
            let mut shuffled = segment.clone();
            for end in (1..=shuffled.len()).rev() {
                let index = stream.index(end)?;
                let last = end
                    .checked_sub(1)
                    .ok_or_else(|| numeric("permutation", "empty swap"))?;
                if index != last {
                    let left = shuffled
                        .get(index)
                        .copied()
                        .ok_or_else(|| numeric("permutation", "index out of bounds"))?;
                    let right = shuffled
                        .get(last)
                        .copied()
                        .ok_or_else(|| numeric("permutation", "index out of bounds"))?;
                    *shuffled
                        .get_mut(index)
                        .ok_or_else(|| numeric("permutation", "index out of bounds"))? = right;
                    *shuffled
                        .get_mut(last)
                        .ok_or_else(|| numeric("permutation", "index out of bounds"))? = left;
                }
            }
            output.extend(shuffled);
        }
        Ok(output)
    }

    fn bca(
        &self,
        replicates: &[f64],
        estimate: f64,
        quantile: Quantile,
        confidence: f64,
    ) -> Result<(f64, f64), String> {
        let n = count(replicates.len()).map_err(|error| error.to_string())?;
        let less = replicates.iter().filter(|value| **value < estimate).count();
        let equal = replicates
            .iter()
            .filter(|value| **value == estimate)
            .count();
        let proportion = (count(less).map_err(|error| error.to_string())?
            + count(equal).map_err(|error| error.to_string())? / 2.0)
            / n;
        if proportion <= 0.0 || proportion >= 1.0 {
            return Err(
                "BCa bias correction is infinite at a degenerate bootstrap endpoint".to_owned(),
            );
        }
        let z0 = normal_quantile(proportion).map_err(|error| error.to_string())?;
        let mut jackknife = Vec::new();
        if self.segments.len() > 1 {
            for deleted in 0..self.segments.len() {
                let sample = self
                    .segments
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| *index != deleted)
                    .flat_map(|(_, segment)| segment.iter().copied())
                    .collect::<Vec<_>>();
                let mut sorted = sample;
                sorted.sort_by(f64::total_cmp);
                if let Some(value) = nearest_rank(&sorted, quantile) {
                    jackknife.push(value);
                }
            }
        } else {
            let segment = self
                .segments
                .first()
                .ok_or_else(|| "no jackknife segment".to_owned())?;
            let length = *self.lengths.first().unwrap_or(&1);
            if segment.len() < 2 {
                return Err("fewer than two observations leave no delete-unit jackknife".to_owned());
            }
            let mut start = 0;
            while start < segment.len() {
                let end = start.saturating_add(length).min(segment.len());
                let sample = segment
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| *index < start || *index >= end)
                    .map(|(_, value)| *value)
                    .collect::<Vec<_>>();
                let mut sorted = sample;
                sorted.sort_by(f64::total_cmp);
                if let Some(value) = nearest_rank(&sorted, quantile) {
                    jackknife.push(value);
                }
                start = end;
            }
        }
        if jackknife.len() < 2 {
            return Err("fewer than two distinct delete-unit jackknife values".to_owned());
        }
        let centre = mean(&jackknife).ok_or_else(|| "empty jackknife mean".to_owned())?;
        let numerator = jackknife
            .iter()
            .map(|value| (centre - value).powi(3))
            .sum::<f64>();
        let squared = jackknife
            .iter()
            .map(|value| (centre - value).powi(2))
            .sum::<f64>();
        if squared == 0.0 {
            return Err("BCa acceleration is undefined for a constant jackknife".to_owned());
        }
        let acceleration = numerator / (6.0 * squared.powf(1.5));
        if !acceleration.is_finite() {
            return Err("BCa acceleration is not finite".to_owned());
        }
        let alpha = (1.0 - confidence) / 2.0;
        let adjusted = |tail: f64| -> Result<f64, String> {
            let z = normal_quantile(tail).map_err(|error| error.to_string())?;
            let denominator = 1.0 - acceleration * (z0 + z);
            if denominator == 0.0 {
                return Err("BCa adjusted endpoint has a zero denominator".to_owned());
            }
            Ok(normal_cdf(z0 + (z0 + z) / denominator)
                .map_err(|error| error.to_string())?
                .clamp(0.0, 1.0))
        };
        Ok((
            order_quantile(replicates, adjusted(alpha)?).map_err(|error| error.to_string())?,
            order_quantile(replicates, adjusted(1.0 - alpha)?)
                .map_err(|error| error.to_string())?,
        ))
    }
}

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
mod tests {
    use super::*;

    #[test]
    fn streams_depend_on_identity_not_row_order() {
        let mut first = SeedStream::new(&["seed", "round", "cell", "p50"]).unwrap();
        let mut second = SeedStream::new(&["seed", "round", "cell", "p50"]).unwrap();
        let mut different = SeedStream::new(&["seed", "round", "other", "p50"]).unwrap();
        let a = first.next_u64().unwrap();
        assert_eq!(a, second.next_u64().unwrap());
        assert_ne!(a, different.next_u64().unwrap());
    }

    #[test]
    fn automatic_blocks_respect_bounds_and_correlation() {
        let independent = (0..100)
            .map(|index| if index % 2 == 0 { 0.0 } else { 1.0 })
            .collect::<Vec<_>>();
        assert!(politis_white(&independent).unwrap().length >= 1);
        let correlated = (0..100)
            .map(|index| f64::from(index / 10))
            .collect::<Vec<_>>();
        let choice = politis_white(&correlated).unwrap();
        assert!(choice.length > 1);
        assert!(choice.length <= correlated.len());
    }

    #[test]
    fn constant_quantile_returns_a_labelled_fallback() {
        let bootstrap = BlockBootstrap::new(vec![vec![4.0; 20]], true).unwrap();
        let mut stream = SeedStream::new(&["fixture"]).unwrap();
        let interval = bootstrap
            .quantile_interval(
                &[4.0; 20],
                Quantile::P50,
                ResamplingContract {
                    resamples: 100,
                    confidence: 0.95,
                },
                &mut stream,
            )
            .unwrap();
        assert_eq!(interval.status, "percentile_fallback");
        assert!(interval.reason.is_some());
        assert_eq!(interval.percentile, (4.0, 4.0));
    }
}
