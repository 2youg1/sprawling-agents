// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The first line of a segment, read without reading the segment.
//!
//! The version probe and the city's identity both need line 1 alone,
//! and a segment grows to tens of megabytes: lifting it whole to split
//! off one line made a single-segment ledger cost two full reads on
//! every open (`crates/storage/spec/Jsonl.lean` §8-1). The window starts at one page and
//! doubles, so a line of any length costs at most twice its own bytes.

use std::io;
use std::path::Path;

use crate::vfs::Vfs;

use super::ledger::u64_count;

/// The first window a reader of a line of unknown length asks for; each
/// next window is twice the last. `jsonl::tail` reads backwards by the
/// same rule (`crates/storage/spec/Jsonl.lean` §8-1).
pub(super) const FIRST_WINDOW_BYTES: u64 = 4096;

/// The first `\n`-terminated line of `path`, without its `\n`.
///
/// `None` when the file holds no terminated line: an empty segment, or
/// one whose first line is torn, which is tail recovery's to judge.
///
/// # Errors
/// Propagates a read the filesystem refuses, and a line longer than a
/// `u64` counts.
pub(crate) fn first_line(vfs: &dyn Vfs, path: &Path) -> io::Result<Option<Vec<u8>>> {
    let mut line = Vec::new();
    let mut window = FIRST_WINDOW_BYTES;
    loop {
        let offset = u64_count(line.len())?;
        let chunk = vfs.read_at(path, offset, window)?;
        if let Some(end) = chunk.iter().position(|byte| *byte == b'\n') {
            line.extend(chunk.iter().take(end));
            return Ok(Some(line));
        }
        if u64_count(chunk.len())? < window {
            return Ok(None);
        }
        line.extend_from_slice(&chunk);
        window = window.saturating_mul(2);
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::super::*;
    use crate::error::StorageError;
    use kernel::{EventDraft, EventKind, Payload, RunId, Seq, TimeMs};
    use std::fs;

    fn draft(kind: EventKind, t: u64) -> EventDraft {
        EventDraft {
            run: RunId::CITY,
            t: TimeMs::new(t),
            who: "city".to_string(),
            addr: None,
            kind,
            data: Payload::empty(),
            ig: false,
        }
    }

    /// With more segments behind it, a first line that carries no version is
    /// damage inside the history, not a torn tail: open refuses at line 1
    /// rather than reading past it (`crates/storage/spec/Jsonl.lean` §8-1).
    #[test]
    fn a_mangled_first_line_with_segments_behind_it_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let (mut ledger, _) = JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
        ledger.set_roll_bytes_for_test(1);
        ledger
            .append_all(vec![
                draft(EventKind::CityInitialized, 0),
                draft(EventKind::RunStarted, 1),
            ])
            .unwrap();
        drop(ledger);
        let first = dir.path().join(segment_file_name(Seq::FIRST));
        fs::write(&first, b"{\"no\":\"version\"}\n").unwrap();

        let refused =
            JsonlLedger::open(dir.path(), TimeMs::new(2)).map(|(ledger, _)| ledger.position());
        assert!(
            matches!(&refused, Err(StorageError::Envelope { path, line: 1, .. }) if *path == first),
            "{refused:?}"
        );
    }

    /// The first line is read whole however far it runs past the first
    /// window, so a long first record is probed for its version rather than
    /// read as torn.
    #[test]
    fn a_first_line_longer_than_the_first_window_is_read_whole() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("segment");
        let line = vec![b'x'; 3 * 4096];
        fs::write(&path, [line.as_slice(), b"\nnext\n"].concat()).unwrap();
        let read = super::first_line(&crate::real_fs::RealFs::new(), &path).unwrap();
        assert_eq!(read, Some(line));
    }
}
