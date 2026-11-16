// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::estimators::Quantile;

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
