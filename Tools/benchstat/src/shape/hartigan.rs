// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Hartigan's descriptive dip statistic.

use crate::error::BenchError;
use crate::estimators::count;

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
