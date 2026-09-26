// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{EventDraft, EventKind, EventRecord, GENESIS_PREV, Payload, RunId, TimeMs};
use proptest::prelude::*;

use super::*;
use crate::jsonl::CheckedLine;

/// A chained ledger of `count` lines; `salt` makes two ledgers of the
/// same length differ at every line.
pub(super) fn chain(count: u64, salt: u64) -> Vec<Vec<u8>> {
    let run = RunId::from_bytes([7u8; 16]);
    let mut prev = GENESIS_PREV;
    (0..count)
        .map(|i| {
            let draft = EventDraft {
                run,
                t: TimeMs::new(i.wrapping_add(salt)),
                who: "tester".to_owned(),
                addr: None,
                kind: EventKind::RunStarted,
                data: Payload::new(serde_json::Map::new()).unwrap(),
                ig: false,
            };
            let line = EventRecord::from_draft(draft, Seq::new(i), prev)
                .canonical_line()
                .unwrap();
            prev = chain_hash(&line);
            line
        })
        .collect()
}

fn walk(mut check: LineCheck, lines: &[Vec<u8>]) -> (Vec<CheckedLine>, LineCheck) {
    let accepted = lines
        .iter()
        .map(|line| check.advance(line).unwrap())
        .collect();
    (accepted, check)
}

#[test]
fn a_written_snapshot_reads_back_whole_and_fits_only_its_own_line() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("snapshot");
    let ours = chain(5, 0);
    let theirs = chain(5, 1_000);
    let snapshot = ChainSnapshot::cut(3, Seq::new(2), &ours[2], b"views".to_vec());

    write_snapshot(&dir, &snapshot).unwrap();

    assert_eq!(
        read_snapshot(&dir).unwrap(),
        StoredSnapshot::Present(snapshot.clone())
    );
    assert_eq!(snapshot.fit(&ours[2]), SnapshotFit::Fits);
    assert_eq!(snapshot.fit(&theirs[2]), SnapshotFit::Stale);
}

#[test]
fn a_flipped_byte_reads_as_damaged_rather_than_as_other_views() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("snapshot");
    let ours = chain(1, 0);
    write_snapshot(
        &dir,
        &ChainSnapshot::cut(1, Seq::FIRST, &ours[0], b"views".to_vec()),
    )
    .unwrap();
    let file = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.is_file())
        .unwrap();
    let mut bytes = std::fs::read(&file).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 1;
    std::fs::write(&file, &bytes).unwrap();

    assert!(matches!(
        read_snapshot(&dir).unwrap(),
        StoredSnapshot::Damaged(_)
    ));
    assert_eq!(
        read_snapshot(&tmp.path().join("none")).unwrap(),
        StoredSnapshot::Absent
    );
}

proptest! {
    /// `snapshotPlusTailIsWhole` with the chain check as the fold: from
    /// any cut, the resumed walk accepts the lines the whole walk accepts
    /// after the cut, and ends in the same chain state.
    #[test]
    fn a_snapshot_plus_its_tail_is_the_whole_walk(count in 1u64..24, cut in any::<prop::sample::Index>()) {
        let lines = chain(count, 0);
        let at = cut.index(lines.len());
        let snapshot = ChainSnapshot::cut(1, Seq::new(u64::try_from(at).unwrap()), &lines[at], Vec::new());

        let (whole, end) = walk(LineCheck::at_genesis(), &lines);
        let (tail, resumed_end) = walk(snapshot.resume().unwrap(), &lines[at + 1..]);

        prop_assert_eq!(&tail[..], &whole[at + 1..]);
        prop_assert_eq!(resumed_end, end);
    }
}
