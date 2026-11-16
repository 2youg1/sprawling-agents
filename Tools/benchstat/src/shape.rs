// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Hartigan's descriptive dip and Sheather–Jones KDE shape summaries.

mod hartigan;
mod kde;

pub use hartigan::{Dip, dip};
pub use kde::sheather_jones;

use crate::error::BenchError;
use crate::estimators::count;
use crate::resampling::{BlockBootstrap, SeedStream};

use kde::{antimodes, basin_of, kde, maxima, shares};

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
    } = supported_basins(
        values,
        &grid,
        &density,
        original_modes.iter().copied().zip(support.iter().copied()),
    )?;
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
///
/// A centre and its support travel together because the threshold decides
/// whether the centre is a mode at all, so pairing them here keeps that rule
/// in one place instead of on every caller.
fn supported_basins(
    values: &[f64],
    grid: &[f64],
    density: &[f64],
    peaks: impl Iterator<Item = (f64, f64)>,
) -> Result<Basins, BenchError> {
    let modes = peaks
        .filter(|(_, support)| *support >= 0.95)
        .map(|(mode, _)| mode)
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
            original
                .iter()
                .copied()
                .zip(std::iter::repeat_n(0.94, original.len())),
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
