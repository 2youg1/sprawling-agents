// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Pure estimators with no filesystem, clock or global state.

use statrs::distribution::{Binomial, ContinuousCDF, Discrete, Normal};

use crate::error::BenchError;

/// The three quantiles bound by the method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Quantile {
    /// Median.
    P50,
    /// 95th percentile.
    P95,
    /// 99th percentile.
    P99,
}

impl Quantile {
    /// Export field spelling.
    pub fn name(self) -> &'static str {
        match self {
            Self::P50 => "p50",
            Self::P95 => "p95",
            Self::P99 => "p99",
        }
    }

    fn ratio(self) -> (u64, u64) {
        match self {
            Self::P50 => (50, 100),
            Self::P95 => (95, 100),
            Self::P99 => (99, 100),
        }
    }
}

/// Hyndman–Fan type 1 over an already sorted non-empty slice.
pub fn nearest_rank(sorted: &[f64], quantile: Quantile) -> Option<f64> {
    let n = u64::try_from(sorted.len()).ok()?;
    let (numerator, denominator) = quantile.ratio();
    let rank = numerator
        .checked_mul(n)?
        .checked_add(denominator.checked_sub(1)?)?
        .checked_div(denominator)?
        .max(1)
        .min(n);
    let index = usize::try_from(rank.checked_sub(1)?).ok()?;
    sorted.get(index).copied()
}

/// Minimum and maximum without hiding an empty input.
pub fn range(values: &[f64]) -> Option<(f64, f64)> {
    let mut iter = values.iter().copied();
    let first = iter.next()?;
    Some(iter.fold((first, first), |(low, high), value| {
        (low.min(value), high.max(value))
    }))
}

/// Arithmetic mean.
pub fn mean(values: &[f64]) -> Option<f64> {
    let n = count(values.len()).ok()?;
    Some(values.iter().sum::<f64>() / n)
}

/// Sample standard deviation with n-1 denominator.
pub fn sample_sd(values: &[f64]) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }
    let centre = mean(values)?;
    let n = count(values.len().checked_sub(1)?).ok()?;
    Some(
        (values
            .iter()
            .map(|value| (value - centre).powi(2))
            .sum::<f64>()
            / n)
            .sqrt(),
    )
}

/// Coefficient of variation; a zero mean has no relative reading.
pub fn coefficient_of_variation(values: &[f64]) -> Option<f64> {
    let centre = mean(values)?;
    if centre == 0.0 {
        None
    } else {
        Some(sample_sd(values)? / centre)
    }
}

/// Tail counts strictly above two and five times P99.
pub fn tail_counts(values: &[f64]) -> Option<(f64, u64, u64)> {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let p99 = nearest_rank(&sorted, Quantile::P99)?;
    if p99 == 0.0 {
        return None;
    }
    let (_, high) = range(&sorted)?;
    let above = |factor: f64| {
        u64::try_from(values.iter().filter(|value| **value > factor * p99).count()).ok()
    };
    Some((high / p99, above(2.0)?, above(5.0)?))
}

/// Larger half-median over the smaller one in collection order.
pub fn split_half_ratio(values: &[f64]) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }
    let half = values.len().checked_div(2)?;
    let first = values.get(..half)?;
    let second = values.get(half..)?;
    let mut first = first.to_vec();
    let mut second = second.to_vec();
    first.sort_by(f64::total_cmp);
    second.sort_by(f64::total_cmp);
    let a = nearest_rank(&first, Quantile::P50)?;
    let b = nearest_rank(&second, Quantile::P50)?;
    if a == 0.0 || b == 0.0 {
        None
    } else {
        Some(a.max(b) / a.min(b))
    }
}

/// Normalized cumulative-sum descriptive reading.
pub fn cusum(values: &[f64]) -> Option<f64> {
    if values.len() < 3 {
        return None;
    }
    let centre = mean(values)?;
    let scale = sample_sd(values)?;
    if scale == 0.0 {
        return Some(0.0);
    }
    let mut running = 0.0;
    let mut largest: f64 = 0.0;
    for value in values.iter().take(values.len().checked_sub(1)?) {
        running += value - centre;
        largest = largest.max(running.abs());
    }
    let n = count(values.len()).ok()?;
    Some(largest / (scale * n.sqrt()))
}

/// Ordinary least squares result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fit {
    /// Change in y per unit x.
    pub slope: f64,
    /// Intercept at x=0.
    pub intercept: f64,
    /// Share of variance explained, absent for a constant y.
    pub r_squared: Option<f64>,
}

/// OLS over independent cohort points.
pub fn fit(points: &[(f64, f64)]) -> Option<Fit> {
    let mut levels = points.iter().map(|point| point.0).collect::<Vec<_>>();
    levels.sort_by(f64::total_cmp);
    levels.dedup_by(|left, right| left == right);
    if points.len() < 4 || levels.len() < 3 {
        return None;
    }
    let xs = points.iter().map(|point| point.0).collect::<Vec<_>>();
    let ys = points.iter().map(|point| point.1).collect::<Vec<_>>();
    let mean_x = mean(&xs)?;
    let mean_y = mean(&ys)?;
    let spread = xs.iter().map(|value| (value - mean_x).powi(2)).sum::<f64>();
    if spread == 0.0 {
        return None;
    }
    let slope = points
        .iter()
        .map(|(x, y)| (x - mean_x) * (y - mean_y))
        .sum::<f64>()
        / spread;
    let intercept = mean_y - slope * mean_x;
    let residual = points
        .iter()
        .map(|(x, y)| (y - (intercept + slope * x)).powi(2))
        .sum::<f64>();
    let total = ys.iter().map(|y| (y - mean_y).powi(2)).sum::<f64>();
    let r_squared = (total != 0.0).then(|| 1.0 - residual / total);
    Some(Fit {
        slope,
        intercept,
        r_squared,
    })
}

/// Wilson score interval for one independent binomial proportion.
pub fn wilson(
    successes: u64,
    trials: u64,
    confidence: f64,
) -> Result<Option<(f64, f64)>, BenchError> {
    if trials == 0 {
        return Ok(None);
    }
    if successes > trials {
        return Err(numeric("Wilson interval", "successes exceed trials"));
    }
    let z = normal_quantile(1.0 - (1.0 - confidence) / 2.0)?;
    let n = float(trials)?;
    let x = float(successes)?;
    let share = x / n;
    let denominator = 1.0 + z.powi(2) / n;
    let centre = (share + z.powi(2) / (2.0 * n)) / denominator;
    let spread =
        z * (share * (1.0 - share) / n + z.powi(2) / (4.0 * n.powi(2))).sqrt() / denominator;
    Ok(Some((
        (centre - spread).max(0.0),
        (centre + spread).min(1.0),
    )))
}

/// One independent binomial proportion's numerator and denominator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Proportion {
    /// Observed successes.
    pub successes: u64,
    /// Eligible trials.
    pub trials: u64,
}

/// Newcombe hybrid-score interval for two independent, unpaired proportions.
pub fn newcombe(
    first: Proportion,
    second: Proportion,
    confidence: f64,
) -> Result<Option<(f64, f64, f64)>, BenchError> {
    if first.trials == 0 || second.trials == 0 {
        return Ok(None);
    }
    let first_interval = wilson(first.successes, first.trials, confidence)?
        .ok_or_else(|| numeric("Newcombe interval", "missing first Wilson interval"))?;
    let second_interval = wilson(second.successes, second.trials, confidence)?
        .ok_or_else(|| numeric("Newcombe interval", "missing second Wilson interval"))?;
    let p1 = float(first.successes)? / float(first.trials)?;
    let p2 = float(second.successes)? / float(second.trials)?;
    let difference = p1 - p2;
    let low =
        difference - ((p1 - first_interval.0).powi(2) + (second_interval.1 - p2).powi(2)).sqrt();
    let high =
        difference + ((first_interval.1 - p1).powi(2) + (p2 - second_interval.0).powi(2)).sqrt();
    Ok(Some((difference, low.max(-1.0), high.min(1.0))))
}

/// Pettitt statistic and the number of observations before its change point.
pub fn pettitt(values: &[f64]) -> Option<(f64, usize)> {
    if values.len() < 4 {
        return None;
    }
    let ranks = ranks(values)?;
    let n = count(values.len()).ok()?;
    let total = ranks.iter().sum::<f64>();
    let mut before = 0.0;
    let mut largest = 0.0_f64;
    let mut change = 0;
    for (index, rank) in ranks.iter().enumerate().take(values.len().checked_sub(1)?) {
        before += rank;
        let t = count(index.checked_add(1)?).ok()?;
        let after = total - before;
        let u = (before / t - after / (n - t)).abs() * t * (n - t) * 2.0 / n;
        if u > largest {
            largest = u;
            change = index.checked_add(1)?;
        }
    }
    Some((largest, change))
}

/// Holm-adjusted p-values in their original order.
pub fn holm(p_values: &[f64]) -> Vec<f64> {
    let mut order = p_values.iter().copied().enumerate().collect::<Vec<_>>();
    order.sort_by(|left, right| left.1.total_cmp(&right.1));
    let m = p_values.len();
    let mut running = 0.0_f64;
    let mut adjusted = vec![1.0; m];
    for (rank, (index, p)) in order.into_iter().enumerate() {
        let factor = m.checked_sub(rank).and_then(|value| count(value).ok());
        if let (Some(slot), Some(factor)) = (adjusted.get_mut(index), factor) {
            running = running.max((factor * p).min(1.0));
            *slot = running;
        }
    }
    adjusted
}

/// Exact independent outcome sizing contract.
#[derive(Debug, Clone, Copy)]
pub struct PowerContract {
    /// Assumed first-arm proportion.
    pub first: f64,
    /// Assumed second-arm proportion.
    pub second: f64,
    /// Two-sided family alpha.
    pub alpha: f64,
    /// Required power.
    pub power: f64,
    /// Multiplicity family size K.
    pub family_size: u32,
    /// Maximum per-arm count.
    pub ceiling: u64,
}

/// Smallest equal-arm count reaching exact score-test power at alpha/K.
pub fn exact_independent_required_n(contract: PowerContract) -> Result<Option<u64>, BenchError> {
    let PowerContract {
        first,
        second,
        alpha,
        power,
        family_size,
        ceiling,
    } = contract;
    if first == second || family_size == 0 {
        return Ok(None);
    }
    let z = normal_quantile(1.0 - alpha / (2.0 * float(family_size.into())?))?;
    let first_dist = |n| {
        Binomial::new(first, n).map_err(|_| numeric("binomial sizing", "invalid first proportion"))
    };
    let second_dist = |n| {
        Binomial::new(second, n)
            .map_err(|_| numeric("binomial sizing", "invalid second proportion"))
    };
    for n in 2..=ceiling {
        let a = first_dist(n)?;
        let b = second_dist(n)?;
        let nf = float(n)?;
        let mut achieved = 0.0;
        for x in 0..=n {
            for y in 0..=n {
                let pooled = float(
                    x.checked_add(y)
                        .ok_or_else(|| numeric("binomial sizing", "count overflow"))?,
                )? / (2.0 * nf);
                if pooled == 0.0 || pooled == 1.0 {
                    continue;
                }
                let difference = float(x)? / nf - float(y)? / nf;
                let statistic = difference.abs() / (pooled * (1.0 - pooled) * 2.0 / nf).sqrt();
                if statistic >= z {
                    achieved += a.pmf(x) * b.pmf(y);
                }
            }
        }
        if achieved >= power {
            return Ok(Some(n));
        }
    }
    Ok(None)
}

/// Convert a bounded count without a lossy cast.
pub fn count(value: usize) -> Result<f64, BenchError> {
    let narrowed =
        u32::try_from(value).map_err(|_| numeric("count conversion", "count exceeds u32"))?;
    Ok(f64::from(narrowed))
}

fn float(value: u64) -> Result<f64, BenchError> {
    let narrowed =
        u32::try_from(value).map_err(|_| numeric("count conversion", "count exceeds u32"))?;
    Ok(f64::from(narrowed))
}

/// Deterministic normal CDF used by BCa.
pub fn normal_cdf(value: f64) -> Result<f64, BenchError> {
    Ok(normal()?.cdf(value))
}

/// Deterministic inverse normal CDF used by BCa and score tests.
pub fn normal_quantile(value: f64) -> Result<f64, BenchError> {
    Ok(normal()?.inverse_cdf(value))
}

fn normal() -> Result<Normal, BenchError> {
    Normal::new(0.0, 1.0).map_err(|_| numeric("normal distribution", "construction failed"))
}

fn ranks(values: &[f64]) -> Option<Vec<f64>> {
    let mut order = values.iter().copied().enumerate().collect::<Vec<_>>();
    order.sort_by(|left, right| left.1.total_cmp(&right.1));
    let mut result = vec![0.0; values.len()];
    let mut start = 0;
    while start < order.len() {
        let value = order.get(start)?.1;
        let mut end = start.checked_add(1)?;
        while order.get(end).is_some_and(|entry| entry.1 == value) {
            end = end.checked_add(1)?;
        }
        let rank = (count(start.checked_add(1)?).ok()? + count(end).ok()?) / 2.0;
        for entry in order.get(start..end)? {
            let slot = result.get_mut(entry.0)?;
            *slot = rank;
        }
        start = end;
    }
    Some(result)
}

fn numeric(action: &str, subject: &str) -> BenchError {
    BenchError::numeric(
        action,
        subject.to_owned(),
        "check the observations and pre-registered parameters",
    )
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    #[test]
    fn nearest_rank_is_the_inverse_empirical_cdf() {
        let values = [1.0, 2.0, 3.0, 100.0];
        assert_eq!(nearest_rank(&values, Quantile::P50), Some(2.0));
        assert_eq!(nearest_rank(&values, Quantile::P95), Some(100.0));
        assert_eq!(nearest_rank(&values, Quantile::P99), Some(100.0));
        assert_eq!(nearest_rank(&[], Quantile::P50), None);
    }

    #[test]
    fn outcome_intervals_match_published_small_sample_values() {
        let (low, high) = wilson(6, 6, 0.95).unwrap().unwrap();
        assert!((low - 0.610).abs() < 0.002);
        assert_eq!(high, 1.0);
        let (low, high) = wilson(5, 6, 0.95).unwrap().unwrap();
        assert!((low - 0.436).abs() < 0.002);
        assert!((high - 0.970).abs() < 0.002);
    }

    #[test]
    fn ols_and_drift_have_closed_form_answers() {
        let points = [(1.0, 3.0), (2.0, 5.0), (3.0, 7.0), (4.0, 9.0)];
        let fitted = fit(&points).unwrap();
        assert_eq!(fitted.slope, 2.0);
        assert_eq!(fitted.intercept, 1.0);
        assert_eq!(fitted.r_squared, Some(1.0));
        let (statistic, change) = pettitt(&[1.0, 1.1, 0.9, 9.0, 9.1, 8.9]).unwrap();
        assert!(statistic > 0.0);
        assert_eq!(change, 3);
    }

    #[test]
    fn holm_is_monotone_in_the_sorted_family() {
        let adjusted = holm(&[0.01, 0.04, 0.02]);
        assert!((adjusted[0] - 0.03).abs() < 1e-12);
        assert!((adjusted[1] - 0.04).abs() < 1e-12);
        assert!((adjusted[2] - 0.04).abs() < 1e-12);
    }
}
