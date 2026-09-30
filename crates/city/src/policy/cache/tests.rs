// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::fs::{self, File};

use kernel::Address;

use super::RulesCache;
use crate::policy::rules_path;

const OPEN: &str = "confidential = false\nwrite = \"everything\"\n";
const SHUT: &str = "confidential = true\nwrite = \"everything\"\n";
// As long as `OPEN`, so only the modification time can tell them apart.
const SHUT_SAME_LENGTH: &str = "confidential = true \nwrite = \"everything\"\n";

fn write_rules(root: &std::path::Path, addr: &Address, text: &str) -> std::path::PathBuf {
    let path = rules_path(root, addr);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, text).unwrap();
    path
}

#[test]
fn an_unchanged_stamp_is_served_without_reading_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let addr = Address::parse("lab").unwrap();
    let path = write_rules(dir.path(), &addr, OPEN);
    let cache = RulesCache::new(dir.path());
    assert!(!cache.load(&addr).unwrap().policy().confidential);

    let stamp = fs::metadata(&path).unwrap().modified().unwrap();
    fs::write(&path, SHUT_SAME_LENGTH).unwrap();
    File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(stamp)
        .unwrap();

    assert!(!cache.load(&addr).unwrap().policy().confidential);
}

#[test]
fn a_changed_length_is_read_again() {
    let dir = tempfile::tempdir().unwrap();
    let addr = Address::parse("lab").unwrap();
    let path = write_rules(dir.path(), &addr, OPEN);
    let cache = RulesCache::new(dir.path());
    assert!(!cache.load(&addr).unwrap().policy().confidential);

    let stamp = fs::metadata(&path).unwrap().modified().unwrap();
    fs::write(&path, SHUT).unwrap();
    File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(stamp)
        .unwrap();

    assert!(cache.load(&addr).unwrap().policy().confidential);
}

/// The instrument behind the `rules_lookup` row of `tools/xtask/budgets.toml`:
/// one rules lookup the way the read bound made it before this cache
/// (a fresh `load`) against the way it makes it now, in alternating
/// rounds so both arms see the same machine state.
#[test]
#[ignore = "a measurement, run by hand with --ignored --nocapture"]
fn rules_lookup_reading() {
    const CALLS: u32 = 2000;
    const ROUNDS: usize = 15;
    let dir = tempfile::tempdir().unwrap();
    let addr = Address::parse("lab").unwrap();
    write_rules(dir.path(), &addr, OPEN);
    let cache = RulesCache::new(dir.path());
    let per_call = |lookup: &dyn Fn() -> bool| {
        let start = std::time::Instant::now();
        for _ in 0..CALLS {
            std::hint::black_box(lookup());
        }
        start.elapsed().as_nanos() / u128::from(CALLS)
    };
    let (mut fresh, mut kept): (Vec<u128>, Vec<u128>) = (0..ROUNDS)
        .map(|_| {
            (
                per_call(&|| {
                    crate::policy::load(dir.path(), &addr)
                        .unwrap()
                        .policy()
                        .confidential
                }),
                per_call(&|| cache.load(&addr).unwrap().policy().confidential),
            )
        })
        .unzip();
    fresh.sort_unstable();
    kept.sort_unstable();
    println!(
        "rules lookup ns/call, median of {ROUNDS} rounds of {CALLS}: load {} | cache {}",
        fresh[ROUNDS / 2],
        kept[ROUNDS / 2]
    );
}
