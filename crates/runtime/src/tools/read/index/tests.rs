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

/// A fill that read the file before a whole read offered newer facts
/// leaves those facts standing when it lands.
#[test]
fn a_fill_overtaken_by_an_offer_keeps_the_offered_entry() {
    let index = Index::new();
    let real = Path::new("/city/a.txt");
    assert!(index.claim(real));
    index.offer(real.to_path_buf(), entry("B", 11));
    index.settle(real.to_path_buf(), Ok(entry("A", 10)));
    assert_eq!(version_at(&index, real, 11).as_deref(), Some("B"));
    assert_eq!(version_at(&index, real, 10), None);
}

/// A fill that started before its path was forgotten installs nothing.
#[test]
fn a_fill_overtaken_by_a_forget_installs_nothing() {
    let index = Index::new();
    let real = Path::new("/city/a.txt");
    assert!(index.claim(real));
    index.forget(real);
    index.settle(real.to_path_buf(), Ok(entry("A", 10)));
    assert_eq!(version_at(&index, real, 10), None);
}

/// A current fill replaces a stale entry, and a failed one removes it.
#[test]
fn a_current_fill_replaces_or_removes_the_stale_entry() {
    let index = Index::new();
    let real = Path::new("/city/a.txt");
    index.offer(real.to_path_buf(), entry("A", 10));
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
