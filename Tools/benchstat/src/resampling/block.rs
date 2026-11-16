// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::error::BenchError;
use crate::estimators::{
    Quantile, count, mean, nearest_rank, normal_cdf, normal_quantile, pettitt,
};

use super::{Interval, ResamplingContract, SeedStream, numeric, order_quantile, politis_white};

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
