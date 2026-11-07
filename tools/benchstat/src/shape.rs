// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Hartigan's descriptive dip and Sheather–Jones KDE shape summaries.

use rustfft::{FftPlanner, num_complex::Complex};

use crate::error::BenchError;
use crate::estimators::{count, sample_sd};
use crate::resampling::{BlockBootstrap, SeedStream, order_quantile};

/// Descriptive dip value and its modal interval.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dip {
    /// The statistic, not a calibrated p-value.
    pub statistic: f64,
    /// Lower modal-interval endpoint.
    pub low: f64,
    /// Upper modal-interval endpoint.
    pub high: f64,
}

/// KDE shape summary on the fixed original grid.
#[derive(Debug, Clone)]
pub struct Shape {
    /// Positive Sheather–Jones STE bandwidth.
    pub bandwidth: f64,
    /// 4097 equally spaced grid coordinates.
    pub grid: Vec<f64>,
    /// Gaussian KDE at each grid coordinate.
    pub density: Vec<f64>,
    /// Original maxima, aligned with bootstrap support.
    pub original_modes: Vec<f64>,
    /// Supported mode centres.
    pub modes: Vec<f64>,
    /// Cuts between retained modes.
    pub antimodes: Vec<f64>,
    /// Empirical shares in the retained basins.
    pub shares: Vec<f64>,
    /// Bootstrap support for each original maximum.
    pub support: Vec<f64>,
    /// Replicate maxima that could not be matched.
    pub unmatched: u64,
}

fn hulls(x: &[f64]) -> Result<(Vec<usize>, Vec<usize>), BenchError> {
    let n = x.len();
    let mut minorant = vec![0_usize; n.saturating_add(2)];
    let mut majorant = vec![0_usize; n.saturating_add(2)];
    set(&mut minorant, 1, 1)?;
    for j in 2..=n {
        set(
            &mut minorant,
            j,
            j.checked_sub(1)
                .ok_or_else(|| numeric("dip", "index underflow"))?,
        )?;
        let mut steps = 0_usize;
        loop {
            steps = steps
                .checked_add(1)
                .ok_or_else(|| numeric("dip", "minorant did not terminate"))?;
            if steps > n {
                return Err(numeric("dip", "minorant iteration bound exceeded"));
            }
            let was = get(&minorant, j)?;
            let before = get(&minorant, was)?;
            if was == 1
                || (at(x, j)? - at(x, was)?)
                    * count(
                        was.checked_sub(before)
                            .ok_or_else(|| numeric("dip", "minorant index underflow"))?,
                    )?
                    < (at(x, was)? - at(x, before)?)
                        * count(
                            j.checked_sub(was)
                                .ok_or_else(|| numeric("dip", "minorant index underflow"))?,
                        )?
            {
                break;
            }
            set(&mut minorant, j, before)?;
        }
    }
    set(&mut majorant, n, n)?;
    for k in (1..n).rev() {
        set(
            &mut majorant,
            k,
            k.checked_add(1)
                .ok_or_else(|| numeric("dip", "index overflow"))?,
        )?;
        let mut steps = 0_usize;
        loop {
            steps = steps
                .checked_add(1)
                .ok_or_else(|| numeric("dip", "majorant did not terminate"))?;
            if steps > n {
                return Err(numeric("dip", "majorant iteration bound exceeded"));
            }
            let was = get(&majorant, k)?;
            let after = get(&majorant, was)?;
            if was == n
                || (at(x, k)? - at(x, was)?) * (count(was)? - count(after)?)
                    < (at(x, was)? - at(x, after)?) * (count(k)? - count(was)?)
            {
                break;
            }
            set(&mut majorant, k, after)?;
        }
    }
    Ok((minorant, majorant))
}

/// Compute Hartigan's AS 217 dip without claiming a dependent-data p-value.
pub fn dip(values: &[f64]) -> Result<Dip, BenchError> {
    let mut x = values.to_vec();
    x.sort_by(f64::total_cmp);
    let n = x.len();
    if n < 2 {
        let only = x.first().copied().unwrap_or(0.0);
        return Ok(Dip {
            statistic: 0.0,
            low: only,
            high: only,
        });
    }
    if x.first() == x.last() {
        let only = x.first().copied().unwrap_or(0.0);
        return Ok(Dip {
            statistic: 0.0,
            low: only,
            high: only,
        });
    }
    let (minorant, majorant) = hulls(&x)?;
    let mut low = 1;
    let mut high = n;
    let mut scaled = 0.0_f64;
    let mut convex = vec![0_usize; n.saturating_add(2)];
    let mut concave = vec![0_usize; n.saturating_add(2)];
    let mut rounds = 0_usize;
    loop {
        rounds = rounds
            .checked_add(1)
            .ok_or_else(|| numeric("dip", "round overflow"))?;
        if rounds > n.saturating_mul(2) {
            return Err(numeric("dip", "modal-interval iteration did not converge"));
        }
        set(&mut convex, 1, high)?;
        let mut i = 1;
        while get(&convex, i)? > low {
            let next = get(&minorant, get(&convex, i)?)?;
            i = i
                .checked_add(1)
                .ok_or_else(|| numeric("dip", "convex index overflow"))?;
            set(&mut convex, i, next)?;
        }
        let convex_length = i;
        let mut convex_at = convex_length
            .checked_sub(1)
            .ok_or_else(|| numeric("dip", "empty convex hull"))?;
        set(&mut concave, 1, low)?;
        i = 1;
        while get(&concave, i)? < high {
            let next = get(&majorant, get(&concave, i)?)?;
            i = i
                .checked_add(1)
                .ok_or_else(|| numeric("dip", "concave index overflow"))?;
            set(&mut concave, i, next)?;
        }
        let concave_length = i;
        let mut concave_at = 2;
        let mut lower_at = convex_length;
        let mut upper_at = concave_length;
        let mut distance = 0.0_f64;
        if convex_length != 2 || concave_length != 2 {
            let mut steps = 0_usize;
            loop {
                steps = steps
                    .checked_add(1)
                    .ok_or_else(|| numeric("dip", "hull walk overflow"))?;
                if steps > n.saturating_mul(2) {
                    return Err(numeric("dip", "hull walk did not terminate"));
                }
                let here = get(&convex, convex_at)?;
                let there = get(&concave, concave_at)?;
                if here > there {
                    let after = get(
                        &convex,
                        convex_at
                            .checked_add(1)
                            .ok_or_else(|| numeric("dip", "convex index overflow"))?,
                    )?;
                    let denominator = at(&x, here)? - at(&x, after)?;
                    if denominator == 0.0 {
                        return Err(numeric("dip", "zero convex slope"));
                    }
                    let step = count(
                        there
                            .checked_sub(after)
                            .and_then(|v| v.checked_add(1))
                            .ok_or_else(|| numeric("dip", "hull index underflow"))?,
                    )? - (at(&x, there)? - at(&x, after)?)
                        * count(
                            here.checked_sub(after)
                                .ok_or_else(|| numeric("dip", "hull index underflow"))?,
                        )?
                        / denominator;
                    concave_at = concave_at
                        .checked_add(1)
                        .ok_or_else(|| numeric("dip", "concave index overflow"))?;
                    if step >= distance {
                        distance = step;
                        lower_at = convex_at
                            .checked_add(1)
                            .ok_or_else(|| numeric("dip", "lower index overflow"))?;
                        upper_at = concave_at
                            .checked_sub(1)
                            .ok_or_else(|| numeric("dip", "upper index underflow"))?;
                    }
                } else {
                    let before = get(
                        &concave,
                        concave_at
                            .checked_sub(1)
                            .ok_or_else(|| numeric("dip", "concave index underflow"))?,
                    )?;
                    let denominator = at(&x, there)? - at(&x, before)?;
                    if denominator == 0.0 {
                        return Err(numeric("dip", "zero concave slope"));
                    }
                    let step = (at(&x, here)? - at(&x, before)?)
                        * count(
                            there
                                .checked_sub(before)
                                .ok_or_else(|| numeric("dip", "hull index underflow"))?,
                        )?
                        / denominator
                        - count(
                            here.checked_sub(before)
                                .and_then(|v| v.checked_sub(1))
                                .ok_or_else(|| numeric("dip", "hull index underflow"))?,
                        )?;
                    convex_at = convex_at
                        .checked_sub(1)
                        .ok_or_else(|| numeric("dip", "convex index underflow"))?;
                    if step >= distance {
                        distance = step;
                        lower_at = convex_at
                            .checked_add(1)
                            .ok_or_else(|| numeric("dip", "lower index overflow"))?;
                        upper_at = concave_at;
                    }
                }
                convex_at = convex_at.max(1);
                concave_at = concave_at.min(concave_length);
                if get(&convex, convex_at)? == get(&concave, concave_at)? {
                    break;
                }
            }
            if distance < scaled {
                break;
            }
        }
        scaled = scaled
            .max(lower_reach(&x, &convex, lower_at, convex_length)?)
            .max(upper_reach(&x, &concave, upper_at, concave_length)?);
        if low == get(&convex, lower_at)? && high == get(&concave, upper_at)? {
            break;
        }
        low = get(&convex, lower_at)?;
        high = get(&concave, upper_at)?;
    }
    Ok(Dip {
        statistic: scaled / (2.0 * count(n)?),
        low: at(&x, low)?,
        high: at(&x, high)?,
    })
}

fn lower_reach(
    x: &[f64],
    convex: &[usize],
    lower_at: usize,
    convex_length: usize,
) -> Result<f64, BenchError> {
    let mut lower = 0.0_f64;
    for j in lower_at..convex_length {
        let mut reached = 1.0_f64;
        let from = get(
            convex,
            j.checked_add(1)
                .ok_or_else(|| numeric("dip", "convex index overflow"))?,
        )?;
        let to = get(convex, j)?;
        if to.saturating_sub(from) > 1 && at(x, to)? != at(x, from)? {
            let slope = count(
                to.checked_sub(from)
                    .ok_or_else(|| numeric("dip", "convex index underflow"))?,
            )? / (at(x, to)? - at(x, from)?);
            for point in from..=to {
                reached = reached.max(
                    count(
                        point
                            .checked_sub(from)
                            .and_then(|v| v.checked_add(1))
                            .ok_or_else(|| numeric("dip", "point index underflow"))?,
                    )? - (at(x, point)? - at(x, from)?) * slope,
                );
            }
        }
        lower = lower.max(reached);
    }
    Ok(lower)
}

fn upper_reach(
    x: &[f64],
    concave: &[usize],
    upper_at: usize,
    concave_length: usize,
) -> Result<f64, BenchError> {
    let mut upper = 0.0_f64;
    for j in upper_at..concave_length {
        let mut reached = 1.0_f64;
        let from = get(concave, j)?;
        let to = get(
            concave,
            j.checked_add(1)
                .ok_or_else(|| numeric("dip", "concave index overflow"))?,
        )?;
        if to.saturating_sub(from) > 1 && at(x, to)? != at(x, from)? {
            let slope = count(
                to.checked_sub(from)
                    .ok_or_else(|| numeric("dip", "concave index underflow"))?,
            )? / (at(x, to)? - at(x, from)?);
            for point in from..=to {
                reached = reached.max(
                    (at(x, point)? - at(x, from)?) * slope - (count(point)? - count(from)? - 1.0),
                );
            }
        }
        upper = upper.max(reached);
    }
    Ok(upper)
}

/// Compute the original KDE and bootstrap support for its maxima.
pub fn shape(
    values: &[f64],
    bootstrap: &BlockBootstrap,
    resamples: u32,
    stream: &mut SeedStream,
) -> Result<Shape, BenchError> {
    let bandwidth = sheather_jones(values)?;
    let (grid, density) = kde(values, bandwidth)?;
    let original_modes = maxima(&grid, &density)?;
    let original_cuts = antimodes(&grid, &density, &original_modes)?;
    let mut matches = vec![0_u32; original_modes.len()];
    let mut unmatched = 0_u64;
    for _ in 0..resamples {
        let sample = bootstrap.draw(stream)?;
        let (_, replicate_density) = kde(&sample, bandwidth)?;
        let replicate_modes = maxima(&grid, &replicate_density)?;
        let mut used = vec![false; replicate_modes.len()];
        for (mode_index, mode) in original_modes.iter().enumerate() {
            let basin = basin_of(*mode, &original_cuts);
            let candidate = replicate_modes
                .iter()
                .enumerate()
                .filter(|(index, candidate)| {
                    !used.get(*index).copied().unwrap_or(true)
                        && basin_of(**candidate, &original_cuts) == basin
                })
                .min_by(|left, right| {
                    let dl = (*left.1 - *mode).abs();
                    let dr = (*right.1 - *mode).abs();
                    dl.total_cmp(&dr).then_with(|| left.1.total_cmp(right.1))
                });
            if let Some((index, _)) = candidate {
                *used
                    .get_mut(index)
                    .ok_or_else(|| numeric("KDE matching", "match index out of bounds"))? = true;
                let slot = matches
                    .get_mut(mode_index)
                    .ok_or_else(|| numeric("KDE matching", "support index out of bounds"))?;
                *slot = slot
                    .checked_add(1)
                    .ok_or_else(|| numeric("KDE matching", "support overflow"))?;
            }
        }
        unmatched = unmatched
            .checked_add(
                u64::try_from(used.iter().filter(|used| !**used).count())
                    .map_err(|_| numeric("KDE matching", "unmatched count overflow"))?,
            )
            .ok_or_else(|| numeric("KDE matching", "unmatched total overflow"))?;
    }
    let support = matches
        .iter()
        .map(|matched| {
            Ok(count(
                usize::try_from(*matched)
                    .map_err(|_| numeric("KDE support", "count conversion"))?,
            )? / count(
                usize::try_from(resamples)
                    .map_err(|_| numeric("KDE support", "count conversion"))?,
            )?)
        })
        .collect::<Result<Vec<_>, BenchError>>()?;
    let Basins {
        modes,
        antimodes,
        shares,
    } = supported_basins(values, &grid, &density, &original_modes, &support)?;
    Ok(Shape {
        original_modes,
        bandwidth,
        grid,
        density,
        modes,
        antimodes,
        shares,
        support,
        unmatched,
    })
}

/// Supported KDE basin geometry and empirical weights.
struct Basins {
    /// Retained centres.
    modes: Vec<f64>,
    /// Boundaries between retained centres.
    antimodes: Vec<f64>,
    /// Weights inside those boundaries.
    shares: Vec<f64>,
}

/// Retain only supported peaks; no retained peak means no basin summary.
fn supported_basins(
    values: &[f64],
    grid: &[f64],
    density: &[f64],
    original_modes: &[f64],
    support: &[f64],
) -> Result<Basins, BenchError> {
    let modes = original_modes
        .iter()
        .zip(support)
        .filter(|(_, support)| **support >= 0.95)
        .map(|(mode, _)| *mode)
        .collect::<Vec<_>>();
    let (antimodes, shares) = if modes.is_empty() {
        (Vec::new(), Vec::new())
    } else {
        let cuts = antimodes(grid, density, &modes)?;
        let weights = shares(values, &cuts)?;
        (cuts, weights)
    };
    Ok(Basins {
        modes,
        antimodes,
        shares,
    })
}

/// Sheather–Jones solve-the-equation bandwidth with bounded bracketing and bisection.
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

fn kde(values: &[f64], bandwidth: f64) -> Result<(Vec<f64>, Vec<f64>), BenchError> {
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
fn maxima(grid: &[f64], density: &[f64]) -> Result<Vec<f64>, BenchError> {
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

fn antimodes(grid: &[f64], density: &[f64], modes: &[f64]) -> Result<Vec<f64>, BenchError> {
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

fn basin_of(value: f64, cuts: &[f64]) -> usize {
    cuts.iter().filter(|cut| value > **cut).count()
}

fn shares(values: &[f64], cuts: &[f64]) -> Result<Vec<f64>, BenchError> {
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

fn at(values: &[f64], one_based: usize) -> Result<f64, BenchError> {
    values
        .get(
            one_based
                .checked_sub(1)
                .ok_or_else(|| numeric("dip", "zero one-based index"))?,
        )
        .copied()
        .ok_or_else(|| numeric("dip", "observation index out of bounds"))
}

fn get(values: &[usize], one_based: usize) -> Result<usize, BenchError> {
    values
        .get(one_based)
        .copied()
        .ok_or_else(|| numeric("dip", "hull index out of bounds"))
}

fn set(values: &mut [usize], one_based: usize, value: usize) -> Result<(), BenchError> {
    *values
        .get_mut(one_based)
        .ok_or_else(|| numeric("dip", "hull write out of bounds"))? = value;
    Ok(())
}

fn numeric(action: &str, subject: &str) -> BenchError {
    BenchError::numeric(
        action,
        subject.to_owned(),
        "inspect the shape observations and bandwidth",
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn unsupported_peaks_do_not_invent_a_single_basin() {
        let values = [1.0, 1.1, 5.0, 5.1];
        let bandwidth = sheather_jones(&values).unwrap();
        let (grid, density) = kde(&values, bandwidth).unwrap();
        let original = maxima(&grid, &density).unwrap();
        assert!(!original.is_empty());
        let Basins {
            modes,
            antimodes: cuts,
            shares,
        } = supported_basins(
            &values,
            &grid,
            &density,
            &original,
            &vec![0.94; original.len()],
        )
        .unwrap();
        assert!(modes.is_empty());
        assert!(cuts.is_empty());
        assert!(shares.is_empty());
    }

    #[test]
    fn constant_and_reference_dips_match_as_217() {
        assert_eq!(dip(&[4.0; 8]).unwrap().statistic, 0.0);
        assert_eq!(dip(&[0.0, 1.0]).unwrap().statistic, 0.0);
        assert!((dip(&[0.0, 0.1, 1.0, 1.1]).unwrap().statistic - 0.225).abs() < 1e-12);
    }

    #[test]
    fn bandwidth_and_grid_are_positive_and_deterministic() {
        let values = [1.0, 1.1, 0.9, 1.2, 5.0, 5.1, 4.9, 5.2];
        let bandwidth = sheather_jones(&values).unwrap();
        assert!(bandwidth > 0.0);
        let (grid, density) = kde(&values, bandwidth).unwrap();
        assert_eq!(grid.len(), 4097);
        assert_eq!(density.len(), 4097);
        assert!(
            density
                .iter()
                .all(|value| value.is_finite() && *value >= 0.0)
        );
        for (index, coordinate) in grid.iter().enumerate().step_by(257) {
            let direct = values
                .iter()
                .map(|value| (-0.5 * ((coordinate - value) / bandwidth).powi(2)).exp())
                .sum::<f64>()
                / (f64::from(u32::try_from(values.len()).unwrap())
                    * bandwidth
                    * (2.0 * std::f64::consts::PI).sqrt());
            let actual = density.get(index).unwrap();
            let tolerance = 1e-6_f64.max(direct.abs() * 1e-3);
            assert!((actual - direct).abs() <= tolerance);
        }
    }

    #[test]
    fn a_deep_kde_valley_keeps_small_densities_and_its_minimum() {
        let (grid, density) = kde(&[0.0, 16.0], 1.0).unwrap();
        let midpoint = grid.iter().position(|value| *value == 8.0).unwrap();
        let actual = *density.get(midpoint).unwrap();
        let direct = (-32.0_f64).exp() / (2.0 * std::f64::consts::PI).sqrt();
        assert!((actual / direct - 1.0).abs() < 0.02);
        let modes = maxima(&grid, &density).unwrap();
        assert_eq!(modes.len(), 2);
        let cuts = antimodes(&grid, &density, &modes).unwrap();
        assert!((cuts.first().unwrap() - 8.0).abs() < 0.05);
    }

    #[test]
    fn a_bimodal_cell_exports_modes_cuts_shares_and_support() {
        let values = (0..40)
            .map(|index| {
                if index % 2 == 0 {
                    1.0 + f64::from(index % 5) * 0.02
                } else {
                    5.0 + f64::from(index % 5) * 0.02
                }
            })
            .collect::<Vec<_>>();
        let bootstrap = BlockBootstrap::new(vec![values.clone()], true).unwrap();
        let mut stream = SeedStream::new(&["fixture", "round", "cell", "shape"]).unwrap();
        let shape = shape(&values, &bootstrap, 100, &mut stream).unwrap();
        assert_eq!(shape.modes.len(), 2);
        assert_eq!(shape.antimodes.len(), 1);
        assert_eq!(shape.shares.len(), 2);
        assert_eq!(shape.support.len(), 2);
        assert!(shape.support.iter().all(|support| *support >= 0.95));
        assert!((shape.shares.iter().sum::<f64>() - 1.0).abs() < 1e-12);
        let cut = shape.antimodes.first().unwrap();
        assert!(*cut > 1.0 && *cut < 5.0);
    }
}
