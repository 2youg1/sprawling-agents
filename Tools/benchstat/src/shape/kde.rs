// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Sheather–Jones bandwidth and Gaussian KDE helpers.

use rustfft::{FftPlanner, num_complex::Complex};

use crate::error::BenchError;
use crate::estimators::{count, sample_sd};
use crate::resampling::order_quantile;

pub fn sheather_jones(values: &[f64]) -> Result<f64, BenchError> {
    if values.len() < 2 {
        return Err(numeric(
            "Sheather-Jones bandwidth",
            "need at least two observations",
        ));
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = count(values.len())?;
    let sd = sample_sd(values)
        .ok_or_else(|| numeric("Sheather-Jones bandwidth", "missing sample deviation"))?;
    let q25 = order_quantile(&sorted, 0.25)?;
    let q75 = order_quantile(&sorted, 0.75)?;
    let scale = sd.min((q75 - q25) / 1.349);
    if scale <= 0.0 || !scale.is_finite() {
        return Err(numeric(
            "Sheather-Jones bandwidth",
            "sample has no positive robust scale",
        ));
    }
    let a = 1.24 * scale * n.powf(-1.0 / 7.0);
    let b = 1.23 * scale * n.powf(-1.0 / 9.0);
    let td = -phi(values, b, 6)?;
    if td <= 0.0 || !td.is_finite() {
        return Err(numeric(
            "Sheather-Jones bandwidth",
            "sample is too sparse to find TD",
        ));
    }
    let alpha2 = 1.357 * (phi(values, a, 4)? / td).powf(1.0 / 7.0);
    if !alpha2.is_finite() {
        return Err(numeric("Sheather-Jones bandwidth", "invalid alpha2"));
    }
    let c1 = 1.0 / (2.0 * std::f64::consts::PI.sqrt() * n);
    let fsd = |h: f64| -> Result<f64, BenchError> {
        Ok((c1 / phi(values, alpha2 * h.powf(5.0 / 7.0), 4)?).powf(0.2) - h)
    };
    let hmax = 1.144 * scale * n.powf(-0.2);
    let mut lower = 0.1 * hmax;
    let mut upper = hmax;
    let tolerance = 0.1 * lower;
    let mut tries = 0_usize;
    while fsd(lower)? * fsd(upper)? > 0.0 {
        tries = tries
            .checked_add(1)
            .ok_or_else(|| numeric("Sheather-Jones bandwidth", "bracket overflow"))?;
        if tries > 99 {
            return Err(numeric(
                "Sheather-Jones bandwidth",
                "no root in the expanded bracket",
            ));
        }
        if tries % 2 == 1 {
            upper *= 1.2;
        } else {
            lower /= 1.2;
        }
    }
    for _ in 0..200 {
        let middle = (lower + upper) / 2.0;
        let value = fsd(middle)?;
        if value.abs() <= tolerance || (upper - lower).abs() <= tolerance {
            return Ok(middle);
        }
        if fsd(lower)? * value <= 0.0 {
            upper = middle;
        } else {
            lower = middle;
        }
    }
    Err(numeric(
        "Sheather-Jones bandwidth",
        "bisection did not converge",
    ))
}

fn phi(values: &[f64], h: f64, order: u32) -> Result<f64, BenchError> {
    if h <= 0.0 {
        return Err(numeric("bandwidth derivative", "non-positive bandwidth"));
    }
    let n = count(values.len())?;
    let mut sum = 0.0;
    for (i, left) in values.iter().enumerate() {
        for right in values.iter().skip(
            i.checked_add(1)
                .ok_or_else(|| numeric("bandwidth derivative", "index overflow"))?,
        ) {
            let delta = ((left - right) / h).powi(2);
            if delta >= 1000.0 {
                continue;
            }
            let term = (-delta / 2.0).exp()
                * if order == 4 {
                    delta.powi(2) - 6.0 * delta + 3.0
                } else {
                    delta.powi(3) - 15.0 * delta.powi(2) + 45.0 * delta - 15.0
                };
            sum += term;
        }
    }
    let diagonal = if order == 4 { 3.0 * n } else { -15.0 * n };
    let power = if order == 4 { 5 } else { 7 };
    Ok((2.0 * sum + diagonal)
        / (n * (n - 1.0) * h.powi(power) * (2.0 * std::f64::consts::PI).sqrt()))
}

pub(super) fn kde(values: &[f64], bandwidth: f64) -> Result<(Vec<f64>, Vec<f64>), BenchError> {
    const POINTS: usize = 4097;
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let min = sorted
        .first()
        .copied()
        .ok_or_else(|| numeric("KDE", "empty sample"))?;
    let max = sorted
        .last()
        .copied()
        .ok_or_else(|| numeric("KDE", "empty sample"))?;
    let low = min - 4.0 * bandwidth;
    let high = max + 4.0 * bandwidth;
    let step = (high - low)
        / count(
            POINTS
                .checked_sub(1)
                .ok_or_else(|| numeric("KDE", "grid underflow"))?,
        )?;
    if step <= 0.0 || !step.is_finite() {
        return Err(numeric("KDE", "invalid grid step"));
    }
    let grid = (0..POINTS)
        .map(|index| Ok(low + count(index)? * step))
        .collect::<Result<Vec<_>, BenchError>>()?;
    let n = count(values.len())?;
    let mut bins = vec![Complex::new(0.0, 0.0); POINTS];
    for value in values {
        let position = (value - low) / step;
        let left = integer_floor(position, POINTS)?;
        let fraction = position - count(left)?;
        if let Some(slot) = bins.get_mut(left) {
            slot.re += (1.0 - fraction) / n;
        }
        if let Some(slot) = bins.get_mut(left.saturating_add(1).min(POINTS - 1)) {
            slot.re += fraction / n;
        }
    }
    let size = POINTS
        .saturating_mul(2)
        .checked_next_power_of_two()
        .ok_or_else(|| numeric("KDE", "FFT size overflow"))?;
    bins.resize(size, Complex::new(0.0, 0.0));
    let mut kernel = vec![Complex::new(0.0, 0.0); size];
    for index in 0..POINTS {
        let distance = count(index)? * step;
        let value = (-0.5 * (distance / bandwidth).powi(2)).exp()
            / (bandwidth * (2.0 * std::f64::consts::PI).sqrt());
        *kernel
            .get_mut(index)
            .ok_or_else(|| numeric("KDE", "kernel index out of bounds"))? =
            Complex::new(value, 0.0);
        if index > 0 {
            *kernel
                .get_mut(
                    size.checked_sub(index)
                        .ok_or_else(|| numeric("KDE", "kernel mirror underflow"))?,
                )
                .ok_or_else(|| numeric("KDE", "kernel mirror out of bounds"))? =
                Complex::new(value, 0.0);
        }
    }
    let mut planner = FftPlanner::<f64>::new();
    let forward = planner.plan_fft_forward(size);
    let inverse = planner.plan_fft_inverse(size);
    forward.process(&mut bins);
    forward.process(&mut kernel);
    for (bin, factor) in bins.iter_mut().zip(&kernel) {
        let re = bin.re * factor.re - bin.im * factor.im;
        let im = bin.re * factor.im + bin.im * factor.re;
        *bin = Complex::new(re, im);
    }
    inverse.process(&mut bins);
    let scale = count(size)?;
    let density = bins
        .iter()
        .take(POINTS)
        .map(|value| (value.re / scale).max(0.0))
        .collect::<Vec<_>>();
    Ok((grid, density))
}

/// Ignore FFT roundoff only for peak detection, preserving valley depths for cuts.
pub(super) fn maxima(grid: &[f64], density: &[f64]) -> Result<Vec<f64>, BenchError> {
    let roundoff = density.iter().copied().fold(0.0_f64, f64::max) * 1e-12;
    let density = density
        .iter()
        .map(|value| if *value <= roundoff { 0.0 } else { *value })
        .collect::<Vec<_>>();
    let mut modes = Vec::new();
    let mut index = 1_usize;
    while index
        .checked_add(1)
        .is_some_and(|next| next < density.len())
    {
        let current = density
            .get(index)
            .copied()
            .ok_or_else(|| numeric("KDE maxima", "density index out of bounds"))?;
        let left = density
            .get(
                index
                    .checked_sub(1)
                    .ok_or_else(|| numeric("KDE maxima", "left neighbour underflow"))?,
            )
            .copied()
            .ok_or_else(|| numeric("KDE maxima", "left neighbour out of bounds"))?;
        let mut end = index;
        while let Some(next) = end.checked_add(1) {
            if density.get(next) != Some(&current) {
                break;
            }
            end = next;
        }
        let right = density
            .get(
                end.checked_add(1)
                    .ok_or_else(|| numeric("KDE maxima", "right neighbour overflow"))?,
            )
            .copied()
            .ok_or_else(|| numeric("KDE maxima", "right neighbour out of bounds"))?;
        if current > left && current > right {
            modes.push(
                *grid
                    .get(index)
                    .ok_or_else(|| numeric("KDE maxima", "grid index out of bounds"))?,
            );
        }
        index = end
            .checked_add(1)
            .ok_or_else(|| numeric("KDE maxima", "grid index overflow"))?;
    }
    Ok(modes)
}

pub(super) fn antimodes(
    grid: &[f64],
    density: &[f64],
    modes: &[f64],
) -> Result<Vec<f64>, BenchError> {
    let mut cuts = Vec::new();
    for pair in modes.windows(2) {
        let left = *pair
            .first()
            .ok_or_else(|| numeric("KDE antimodes", "missing left mode"))?;
        let right = *pair
            .get(1)
            .ok_or_else(|| numeric("KDE antimodes", "missing right mode"))?;
        let mut best: Option<(f64, f64)> = None;
        for (coordinate, value) in grid.iter().zip(density) {
            if *coordinate >= left
                && *coordinate <= right
                && best.is_none_or(|(best_value, _)| *value < best_value)
            {
                best = Some((*value, *coordinate));
            }
        }
        cuts.push(
            best.ok_or_else(|| numeric("KDE antimodes", "empty basin"))?
                .1,
        );
    }
    Ok(cuts)
}

pub(super) fn basin_of(value: f64, cuts: &[f64]) -> usize {
    cuts.iter().filter(|cut| value > **cut).count()
}

pub(super) fn shares(values: &[f64], cuts: &[f64]) -> Result<Vec<f64>, BenchError> {
    let total = count(values.len())?;
    (0..=cuts.len())
        .map(|basin| {
            let in_basin = values
                .iter()
                .filter(|value| basin_of(**value, cuts) == basin)
                .count();
            if total == 0.0 {
                Ok(0.0)
            } else {
                Ok(count(in_basin)? / total)
            }
        })
        .collect()
}

fn integer_floor(value: f64, ceiling: usize) -> Result<usize, BenchError> {
    for candidate in 0..ceiling {
        if count(
            candidate
                .checked_add(1)
                .ok_or_else(|| numeric("KDE binning", "grid index overflow"))?,
        )? > value
        {
            return Ok(candidate);
        }
    }
    ceiling
        .checked_sub(1)
        .ok_or_else(|| numeric("KDE binning", "empty grid"))
}

fn numeric(action: &str, subject: &str) -> BenchError {
    BenchError::numeric(
        action,
        subject.to_owned(),
        "inspect the shape observations and bandwidth",
    )
}
