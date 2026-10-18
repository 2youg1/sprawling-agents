// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn entry(version: &str, stamp: u64) -> Entry {
    Entry {
        size: 4,
        modified: SystemTime::UNIX_EPOCH
            .checked_add(std::time::Duration::from_secs(stamp))
            .unwrap(),
        total_lines: 1,
        version: version.to_owned(),
        line_offsets: vec![0],
    }
}

fn version_at(index: &Index, real: &Path, stamp: u64) -> Option<String> {
    let probe = entry("", stamp);
    index
        .lookup(real, probe.size, probe.modified, 0)
        .map(|hit| hit.version)
}

/// A fill claimed before a whole read looked at the file leaves that
/// read's offer standing, whichever of the two finishes first.
#[test]
fn an_offer_seen_after_a_fill_was_claimed_wins_in_either_order() {
    let index = Index::new();
    let real = Path::new("/city/a.txt");
    assert!(index.claim(real));
    let seen = index.observe().unwrap();
    index.offer(real.to_path_buf(), entry("B", 11), seen);
    index.settle(real.to_path_buf(), Ok(entry("A", 10)));
    assert_eq!(version_at(&index, real, 11).as_deref(), Some("B"));

    let real = Path::new("/city/b.txt");
    assert!(index.claim(real));
    let seen = index.observe().unwrap();
    index.settle(real.to_path_buf(), Ok(entry("A", 10)));
    index.offer(real.to_path_buf(), entry("B", 11), seen);
    assert_eq!(version_at(&index, real, 11).as_deref(), Some("B"));
}

/// A whole read that looked at the file before a fill was claimed does
/// not displace the fill's facts, whichever of the two finishes first.
#[test]
fn an_offer_seen_before_a_fill_was_claimed_loses_in_either_order() {
    let index = Index::new();
    let real = Path::new("/city/a.txt");
    let seen = index.observe().unwrap();
    assert!(index.claim(real));
    index.offer(real.to_path_buf(), entry("A", 10), seen);
    index.settle(real.to_path_buf(), Ok(entry("B", 11)));
    assert_eq!(version_at(&index, real, 11).as_deref(), Some("B"));

    let real = Path::new("/city/b.txt");
    let seen = index.observe().unwrap();
    assert!(index.claim(real));
    index.settle(real.to_path_buf(), Ok(entry("B", 11)));
    index.offer(real.to_path_buf(), entry("A", 10), seen);
    assert_eq!(version_at(&index, real, 11).as_deref(), Some("B"));
}

/// A table a seek rejected does not come back from a fill that was on
/// its way before the rejection.
#[test]
fn a_fill_claimed_before_a_forget_installs_nothing() {
    let index = Index::new();
    let real = Path::new("/city/a.txt");
    assert!(index.claim(real));
    let seen = index.observe().unwrap();
    index.offer(real.to_path_buf(), entry("A", 10), seen);
    index.forget(real, seen);
    index.settle(real.to_path_buf(), Ok(entry("A", 10)));
    assert_eq!(version_at(&index, real, 10), None);
}

/// A fill claimed after the rejected facts were seen may hold the new
/// file, so a forget leaves it to land.
#[test]
fn a_fill_claimed_after_the_rejected_facts_still_lands() {
    let index = Index::new();
    let real = Path::new("/city/a.txt");
    let seen = index.observe().unwrap();
    index.offer(real.to_path_buf(), entry("A", 10), seen);
    assert!(index.claim(real));
    index.forget(real, seen);
    index.settle(real.to_path_buf(), Ok(entry("B", 11)));
    assert_eq!(version_at(&index, real, 11).as_deref(), Some("B"));
}

/// A forget drops only the entry it rejected, not one that replaced it.
#[test]
fn a_forget_leaves_a_newer_entry_standing() {
    let index = Index::new();
    let real = Path::new("/city/a.txt");
    let old = index.observe().unwrap();
    index.offer(real.to_path_buf(), entry("A", 10), old);
    let new = index.observe().unwrap();
    index.offer(real.to_path_buf(), entry("B", 11), new);
    index.forget(real, old);
    assert_eq!(version_at(&index, real, 11).as_deref(), Some("B"));
}

/// A fill replaces a stale entry seen before it, and a failed one
/// removes it.
#[test]
fn a_fill_replaces_or_removes_an_older_entry() {
    let index = Index::new();
    let real = Path::new("/city/a.txt");
    let seen = index.observe().unwrap();
    index.offer(real.to_path_buf(), entry("A", 10), seen);
    assert!(index.claim(real));
    index.settle(real.to_path_buf(), Ok(entry("B", 11)));
    assert_eq!(version_at(&index, real, 11).as_deref(), Some("B"));

    assert!(index.claim(real));
    let refused = AxError::failure(AxCode::StorageFatal, "read", "gone").with_recovery("r");
    index.settle(real.to_path_buf(), Err(refused));
    assert_eq!(version_at(&index, real, 11), None);
}

/// One fill per path, and no more than the cap at once.
#[test]
fn claims_dedupe_by_path_and_stop_at_the_cap() {
    let index = Index::new();
    assert!(index.claim(Path::new("/city/a.txt")));
    assert!(!index.claim(Path::new("/city/a.txt")));
    assert!(index.claim(Path::new("/city/b.txt")));
    assert!(!index.claim(Path::new("/city/c.txt")));
}
