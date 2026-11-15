// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Mei JSONL persistence regressions, including injected write and rollback
//! failures through the production append seam (`Console.lean` §8-174 D111).

use super::*;

struct FailingFile {
    cursor: std::io::Cursor<Vec<u8>>,
    remaining: usize,
    fail_truncate: bool,
    truncated_to: Option<u64>,
}

impl FailingFile {
    fn new(bytes: Vec<u8>, remaining: usize, fail_truncate: bool) -> FailingFile {
        FailingFile {
            cursor: std::io::Cursor::new(bytes),
            remaining,
            fail_truncate,
            truncated_to: None,
        }
    }
}

impl Read for FailingFile {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.cursor.read(buffer)
    }
}

impl Seek for FailingFile {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        self.cursor.seek(position)
    }
}

impl Write for FailingFile {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        if self.remaining == 0 {
            return Err(std::io::Error::other("injected append failure"));
        }
        let count = buffer.len().min(self.remaining);
        let written = self.cursor.write(&buffer[..count])?;
        self.remaining = self.remaining.saturating_sub(written);
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.cursor.flush()
    }
}

impl SessionFile for FailingFile {
    fn truncate(&mut self, length: u64) -> std::io::Result<()> {
        if self.fail_truncate {
            return Err(std::io::Error::other("injected rollback failure"));
        }
        self.cursor
            .get_mut()
            .truncate(usize::try_from(length).unwrap());
        self.truncated_to = Some(length);
        Ok(())
    }
}

#[test]
fn an_import_without_a_final_newline_stays_parseable_after_append() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("session.jsonl");
    std::fs::write(
        &path,
        concat!(
            "{\"role\":\"user\",\"content\":\"first\"}\n",
            "{\"role\":\"assistant\",\"content\":\"done\",\"stop_reason\":\"end_turn\"}"
        ),
    )
    .unwrap();
    let mut session = MeiSession::import(&path).unwrap();
    session
        .append(&[
            "{\"role\":\"user\",\"content\":\"next\"}".to_owned(),
            "{\"role\":\"assistant\",\"content\":\"again\",\"stop_reason\":\"end_turn\"}"
                .to_owned(),
        ])
        .unwrap();
    let recovered = mei::resume::read(&path).unwrap();
    assert_eq!(recovered.len(), 4);
    assert_eq!(recovered[2]["content"], "next");
    assert_eq!(recovered[3]["content"], "again");
}

#[test]
fn a_failed_append_restores_the_original_bytes() {
    let original = b"{\"role\":\"assistant\",\"content\":\"old\"}\n".to_vec();
    let original_len = u64::try_from(original.len()).unwrap();
    let mut file = FailingFile::new(original.clone(), 5, false);
    let failure = append_contents(
        &mut file,
        &["{\"role\":\"user\",\"content\":\"new\"}".to_owned()],
    )
    .unwrap_err();
    assert!(failure.rollback.is_none());
    assert_eq!(file.truncated_to, Some(original_len));
    assert_eq!(file.cursor.into_inner(), original);
}

#[test]
fn a_failed_rollback_is_not_reported_as_a_successful_restore() {
    let original = b"old\n".to_vec();
    let mut file = FailingFile::new(original, 2, true);
    let failure = append_contents(&mut file, &["new".to_owned()]).unwrap_err();
    assert_eq!(failure.error.to_string(), "injected append failure");
    assert_eq!(
        failure.rollback.map(|error| error.to_string()),
        Some("injected rollback failure".to_owned())
    );
}

#[test]
fn new_session_files_are_unique_and_leave_the_old_bytes_alone() {
    let dir = tempfile::tempdir().unwrap();
    let mut first = MeiSession::create_in(dir.path(), TimeMs::new(100)).unwrap();
    first
        .append(&[
            "{\"role\":\"user\",\"content\":\"first\"}".to_owned(),
            "{\"role\":\"assistant\",\"content\":\"done\",\"stop_reason\":\"end_turn\"}".to_owned(),
        ])
        .unwrap();
    let first_path = first.path().to_path_buf();
    let before = std::fs::read(&first_path).unwrap();
    let second = MeiSession::create_in(dir.path(), TimeMs::new(100)).unwrap();
    assert_ne!(first_path, second.path());
    assert_eq!(std::fs::read(&first_path).unwrap(), before);
    assert_eq!(std::fs::read(second.path()).unwrap(), b"");

    let mut resumed = MeiSession::import(&first_path).unwrap();
    assert_eq!(resumed.history(), first.history());
    assert_eq!(resumed.resume_path(), Some(first_path.as_path()));
    resumed
        .append(&["{\"role\":\"user\",\"content\":\"next\"}".to_owned()])
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(first_path).unwrap().lines().count(),
        3
    );
}
