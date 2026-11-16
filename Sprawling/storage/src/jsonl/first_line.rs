// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io;
use std::path::{Path, PathBuf};

use kernel::consts_external::{LogVersion, readable_log_v};
use kernel::{AxCode, AxError};

use crate::error::{StorageError, io_err};
use crate::vfs::Vfs;

use super::ledger::{JsonlLedger, u64_count};
use super::open::unversioned;

pub(crate) const FIRST_WINDOW_BYTES: u64 = 4096;

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

impl JsonlLedger {

    pub(super) fn probe_version(&mut self, segments: &[PathBuf]) -> Result<(), StorageError> {
        let Some(first) = segments.first() else {
            return Ok(());
        };
        let Some(first_line) =
            first_line(self.vfs.as_ref(), first).map_err(io_err("read segment", first))?
        else {

            return Ok(());
        };

        let probed = serde_json::from_slice::<serde_json::Value>(&first_line)
            .ok()
            .and_then(|value| value.get("v").and_then(serde_json::Value::as_u64));
        let v = match probed {
            Some(v) => v,
            None if segments.len() > 1 => {
                return Err(StorageError::Envelope {
                    path: first.clone(),
                    line: 1,
                    source: AxError::failure(
                        AxCode::InvalidArgs,
                        "probe ledger version",
                        "first line is not a version-bearing record",
                    )
                    .with_recovery(
                        "restore this segment from its checkpoint commit, then run \
                         `sprawling replay <ledger-dir>`: line 1 of every segment carries `v`",
                    ),
                });
            }
            None => return Ok(()),
        };
        match readable_log_v(v) {

            LogVersion::Current | LogVersion::Older => Ok(()),
            LogVersion::Ahead => Err(StorageError::VersionAhead {
                path: first.clone(),
                v,
            }),
            LogVersion::NotAVersion => Err(StorageError::Envelope {
                path: first.clone(),
                line: 1,
                source: unversioned(v),
            }),
        }
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
