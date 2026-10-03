// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The three properties of `crates/sprawling/spec/Serving/Memory.lean`
//! §8-173, checked after every step of an arbitrary sequence of inserts,
//! reads, evictions and freezes: the resident bytes never exceed the
//! budget, a frozen run holds nothing, and every read answers the bytes
//! on disk. Values run up to twice the budget, so an entry larger than
//! the whole budget is drawn.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use std::collections::BTreeMap;
use std::convert::Infallible;

use proptest::prelude::*;

use super::Resident;

const BUDGET: usize = 64;

#[derive(Debug, Clone)]
enum Op {
    Insert(u8, u8),
    Read(u8, u8),
    Evict,
    Freeze(u8),
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        3 => (0..4_u8, 0..16_u8).prop_map(|(run, key)| Op::Insert(run, key)),
        3 => (0..4_u8, 0..16_u8).prop_map(|(run, key)| Op::Read(run, key)),
        1 => Just(Op::Evict),
        1 => (0..4_u8).prop_map(Op::Freeze),
    ]
}

fn disk() -> impl Strategy<Value = BTreeMap<u8, Vec<u8>>> {
    proptest::collection::vec(proptest::collection::vec(any::<u8>(), 0..2 * BUDGET), 16)
        .prop_map(|values| (0..16_u8).zip(values).collect())
}

proptest! {
    #[test]
    fn resident_cache_keeps_the_budget_the_frozen_runs_and_the_disk_bytes(
        disk in disk(),
        ops in proptest::collection::vec(op(), 0..64),
    ) {
        let load = |key: &u8| Ok::<_, Infallible>(disk[key].clone());
        let mut cache = Resident::<u8, u8>::new(BUDGET);
        let mut frozen = Vec::new();
        for step in ops {
            match step {
                Op::Insert(run, key) => cache.insert(&run, &key, load).unwrap(),
                Op::Read(run, key) => {
                    let bytes = cache.read(&run, &key, load).unwrap();
                    prop_assert_eq!(&*bytes, disk[&key].as_slice());
                }
                Op::Evict => cache.evict_oldest(),
                Op::Freeze(run) => {
                    cache.freeze(&run);
                    frozen.push(run);
                }
            }
            let held: usize = cache.bytes.values().map(|bytes| bytes.len()).sum();
            prop_assert_eq!(cache.resident_bytes(), held);
            prop_assert!(held <= BUDGET, "{held} bytes resident over a budget of {BUDGET}");
            prop_assert!(cache.bytes.keys().all(|(run, _)| !frozen.contains(run)));
            prop_assert!(cache.bytes.iter().all(|((_, key), bytes)| **bytes == *disk[key]));
        }
    }
}
