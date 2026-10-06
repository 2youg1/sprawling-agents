// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Strict local JSONL: a read that never creates or settles anything, and
//! a writer that holds the file for a whole operation and returns only once
//! a line is on disk (`crates/sprawling/spec/Privacy/Journal.lean`).

use std::fs::{File, OpenOptions, TryLockError};
use std::io::{ErrorKind, Read, Write};
use std::path::Path;

use super::fault::HistoryFault;
use super::state::{History, Line};

/// History input bound, not a claim about Windows registry value limits.
const HISTORY_BYTES_MAX: u64 = 8 * 1024 * 1024;

pub(super) fn read(path: &Path) -> Result<History, HistoryFault> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(source) if source.kind() == ErrorKind::NotFound => return Ok(History::default()),
        Err(source) => return Err(HistoryFault::Io(source)),
    };
    match file.try_lock_shared() {
        Ok(()) => (),
        Err(TryLockError::WouldBlock) => return Err(HistoryFault::Busy),
        Err(TryLockError::Error(source)) => return Err(HistoryFault::Io(source)),
    }
    let history = decode(&read_bounded(&file)?)?;
    drop(file);
    Ok(history)
}

/// The history held for writing: an exclusive lock on the file, taken
/// before its bytes are read and released on drop, so the fold the
/// caller decides from is the file every append extends.
#[cfg_attr(
    not(any(test, windows)),
    expect(
        dead_code,
        reason = "the coordinator is its one writer and runs on Windows only"
    )
)]
pub(super) struct LockedJournal {
    file: File,
    bytes: Vec<u8>,
    history: History,
}

#[cfg_attr(
    not(any(test, windows)),
    expect(
        dead_code,
        reason = "the coordinator is its one writer and runs on Windows only"
    )
)]
impl LockedJournal {
    /// Opens `path` under an exclusive lock, creating the file and its
    /// directory when absent and syncing the directory that gained an
    /// entry. An existing history is read and folded first; one that does
    /// not decode is refused and left byte for byte as it was.
    ///
    /// # Errors
    /// `Busy` while another process holds the file, `Io` on any file
    /// failure, and every refusal [`read`] gives for the same bytes.
    pub(super) fn open(path: &Path) -> Result<Self, HistoryFault> {
        let file = open_or_create(path)?;
        match file.try_lock() {
            Ok(()) => (),
            Err(TryLockError::WouldBlock) => return Err(HistoryFault::Busy),
            Err(TryLockError::Error(source)) => return Err(HistoryFault::Io(source)),
        }
        let bytes = read_bounded(&file)?;
        let history = decode(&bytes)?;
        Ok(Self {
            file,
            bytes,
            history,
        })
    }

    pub(super) fn history(&self) -> &History {
        &self.history
    }

    /// Appends `line` and returns once the file holds it on disk. The
    /// history with the line added must pass the same decoder a read
    /// uses; a line it would refuse is never written.
    ///
    /// # Errors
    /// The fold's refusal or `Capacity`, with nothing written; `Io` when
    /// the write or the sync fails, after which the caller writes nothing
    /// to the host and a later read reports whatever reached the file.
    pub(super) fn append_durable(&mut self, line: &Line) -> Result<(), HistoryFault> {
        let mut encoded =
            serde_json::to_vec(line).map_err(|source| HistoryFault::Io(source.into()))?;
        encoded.push(b'\n');
        let mut extended = self.bytes.clone();
        extended.extend_from_slice(&encoded);
        if exceeds_capacity(&extended)? {
            return Err(HistoryFault::Capacity);
        }
        let history = decode(&extended)?;
        (&self.file)
            .write_all(&encoded)
            .and_then(|()| self.file.sync_all())
            .map_err(HistoryFault::Io)?;
        self.bytes = extended;
        self.history = history;
        Ok(())
    }
}

/// The history file, opened for reading and appending. A file created
/// here makes its directory's entry durable before any line is written.
fn open_or_create(path: &Path) -> Result<File, HistoryFault> {
    let directory = path
        .parent()
        .ok_or(HistoryFault::Invalid("history path has no directory"))?;
    let mut options = OpenOptions::new();
    options.read(true).append(true);
    match options.open(path) {
        Ok(file) => return Ok(file),
        Err(source) if source.kind() == ErrorKind::NotFound => (),
        Err(source) => return Err(HistoryFault::Io(source)),
    }
    let created_directory = !directory.is_dir();
    std::fs::create_dir_all(directory).map_err(HistoryFault::Io)?;
    let file = options
        .create_new(true)
        .open(path)
        .map_err(HistoryFault::Io)?;
    sync_directory(directory)?;
    if created_directory && let Some(above) = directory.parent() {
        sync_directory(above)?;
    }
    Ok(file)
}

/// Makes a directory's entries durable. Windows flushes a directory only
/// through a handle opened with backup semantics and write access.
fn sync_directory(directory: &Path) -> Result<(), HistoryFault> {
    #[cfg(windows)]
    let handle = {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
        OpenOptions::new()
            .write(true)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(directory)
    };
    #[cfg(not(windows))]
    let handle = File::open(directory);
    handle
        .and_then(|handle| handle.sync_all())
        .map_err(HistoryFault::Io)
}

fn read_bounded(file: &File) -> Result<Vec<u8>, HistoryFault> {
    let mut bytes = Vec::new();
    file.take(HISTORY_BYTES_MAX.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(HistoryFault::Io)?;
    if exceeds_capacity(&bytes)? {
        return Err(HistoryFault::Capacity);
    }
    Ok(bytes)
}

fn exceeds_capacity(bytes: &[u8]) -> Result<bool, HistoryFault> {
    u64::try_from(bytes.len())
        .map(|length| length > HISTORY_BYTES_MAX)
        .map_err(|source| HistoryFault::Io(std::io::Error::other(source)))
}

fn decode(bytes: &[u8]) -> Result<History, HistoryFault> {
    if bytes.is_empty() {
        return Ok(History::default());
    }
    if bytes.last() != Some(&b'\n') {
        return Err(HistoryFault::Invalid("history ends in an incomplete line"));
    }
    let mut lines = Vec::new();
    for line in bytes.split_inclusive(|byte| *byte == b'\n') {
        lines.push(serde_json::from_slice::<Line>(line).map_err(|source| {
            HistoryFault::Decode {
                line: source.line(),
                column: source.column(),
                category: source.classify(),
            }
        })?);
    }
    History::fold(lines)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use super::super::state::fixtures::{apply, bytes, dword, finished, prepared};
    use super::super::state::{Outcome, SCHEMA};
    use super::super::target::{RawValue, Snapshot};
    use super::*;
    use wire::PrivacyControl;

    const PRIVATE_INPUT: &str = "fixture-private-principal";
    const CONTROL: PrivacyControl = PrivacyControl::PowershellTelemetryOptout;

    fn absent() -> Snapshot {
        Snapshot::Registry(RawValue::Absent)
    }

    #[test]
    fn reading_missing_history_creates_nothing() {
        let home = tempfile::tempdir().unwrap();
        let path = accounting::home::Home::at(home.path()).privacy_history();
        assert!(
            read(&path)
                .unwrap()
                .disclose(|_| Ok(()))
                .unwrap()
                .is_empty()
        );
        assert_eq!(std::fs::read_dir(home.path()).unwrap().count(), 0);
    }

    #[test]
    fn reading_unfinished_history_never_settles_or_repairs_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture.jsonl");
        let before = bytes(&[prepared(&apply(1, CONTROL, absent(), dword(1)))]);
        std::fs::write(&path, &before).unwrap();
        let status =
            serde_json::to_value(read(&path).unwrap().disclose(|_| Ok(())).unwrap()).unwrap();
        assert_eq!(
            status,
            serde_json::json!([{ "operation": 1, "outcome": "unresolved" }])
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
        let mut torn = before.clone();
        torn.extend_from_slice(b"{\"schema\":");
        std::fs::write(&path, &torn).unwrap();
        assert!(matches!(read(&path), Err(HistoryFault::Invalid(_))));
        assert_eq!(std::fs::read(&path).unwrap(), torn);
    }

    #[test]
    fn history_read_refuses_an_exclusive_writer() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture.jsonl");
        let writer = File::create(&path).unwrap();
        writer.try_lock().unwrap();
        assert!(matches!(read(&path), Err(HistoryFault::Busy)));
        drop(writer);
        assert!(
            read(&path)
                .unwrap()
                .disclose(|_| Ok(()))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn unknown_or_malformed_history_is_not_empty_success() {
        for input in [
            b"\n".as_slice(),
            b"{}\n",
            b"{\"schema\":999,\"event\":{\"phase\":\"future\"}}\n",
            b"\xff\n",
        ] {
            assert!(decode(input).is_err());
        }
        let change = apply(1, CONTROL, dword(7), dword(1));
        assert!(decode(&bytes(&[finished(&change, Outcome::Applied)])).is_err());
        assert!(
            decode(&bytes(&[
                prepared(&change),
                finished(&change, Outcome::Unknown),
                prepared(&change)
            ]))
            .is_err()
        );
    }

    #[test]
    fn unknown_fields_duplicate_ids_and_cross_identity_history_are_refused() {
        let first = apply(1, CONTROL, dword(7), dword(1));
        let mut other = apply(2, CONTROL, dword(7), dword(1));
        other.owner = kernel::SecretRef::new("privacy", "another-fixture-owner").unwrap();
        assert!(
            decode(&bytes(&[
                prepared(&first),
                finished(&first, Outcome::Applied),
                prepared(&other)
            ]))
            .is_err()
        );
        assert!(
            decode(&bytes(&[
                prepared(&first),
                finished(&first, Outcome::Applied),
                prepared(&first)
            ]))
            .is_err()
        );
        let mut value = serde_json::to_value(prepared(&first)).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("unexpected".to_owned(), serde_json::json!(true));
        let mut line = serde_json::to_vec(&value).unwrap();
        line.push(b'\n');
        assert!(decode(&line).is_err());
    }

    #[test]
    fn plaintext_identity_and_old_schema_are_refused_without_rewriting() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture.jsonl");
        let prepared = prepared(&apply(1, CONTROL, dword(7), dword(1)));
        for (schema, owner) in [(SCHEMA, PRIVATE_INPUT), (2, "secret:privacy/fixture-owner")] {
            let mut line = serde_json::to_value(&prepared).unwrap();
            *line.pointer_mut("/schema").unwrap() = serde_json::json!(schema);
            *line.pointer_mut("/event/intent/owner").unwrap() = serde_json::json!(owner);
            let mut before = serde_json::to_vec(&line).unwrap();
            before.push(b'\n');
            std::fs::write(&path, &before).unwrap();
            assert!(read(&path).is_err(), "unsafe history was accepted");
            assert_eq!(std::fs::read(&path).unwrap(), before);
        }
    }

    #[test]
    fn decoding_refusal_never_repeats_private_input() {
        let prepared = prepared(&apply(1, CONTROL, dword(7), dword(1)));
        for field in ["control", "unexpected", "owner"] {
            let mut line = serde_json::to_value(&prepared).unwrap();
            if field == "unexpected" {
                line.pointer_mut("/event/intent")
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .insert(PRIVATE_INPUT.to_owned(), serde_json::json!(true));
            } else {
                *line.pointer_mut(&format!("/event/intent/{field}")).unwrap() =
                    serde_json::json!(PRIVATE_INPUT);
            }
            let mut before = serde_json::to_vec(&line).unwrap();
            before.push(b'\n');
            let fault = match decode(&before) {
                Ok(_) => panic!("private input accepted"),
                Err(fault) => fault,
            };
            assert!(!format!("{fault:?}").contains(PRIVATE_INPUT));
            let fault = fault.into_ax("read local privacy history");
            assert!(!fault.to_string().contains(PRIVATE_INPUT));
            assert_eq!(fault.code(), &kernel::AxCode::StorageFatal);
            assert!(fault.subject().contains("column"));
        }
    }

    #[test]
    fn an_appended_line_is_in_the_file_when_the_append_returns() {
        let home = tempfile::tempdir().unwrap();
        let path = accounting::home::Home::at(home.path()).privacy_history();
        let intent = apply(1, CONTROL, absent(), dword(1));
        let mut journal = LockedJournal::open(&path).unwrap();
        journal.append_durable(&prepared(&intent)).unwrap();
        let first = bytes(&[prepared(&intent)]);
        assert_eq!(journal.bytes, first);
        assert_eq!(
            serde_json::to_value(journal.history().disclose(|_| Ok(())).unwrap()).unwrap(),
            serde_json::json!([{ "operation": 1, "outcome": "unresolved" }])
        );
        drop(journal);
        assert_eq!(std::fs::read(&path).unwrap(), first);
        let mut journal = LockedJournal::open(&path).unwrap();
        journal
            .append_durable(&finished(&intent, Outcome::Applied))
            .unwrap();
        drop(journal);
        let both = bytes(&[prepared(&intent), finished(&intent, Outcome::Applied)]);
        assert_eq!(std::fs::read(&path).unwrap(), both);
        assert_eq!(
            serde_json::to_value(read(&path).unwrap().disclose(|_| Ok(())).unwrap()).unwrap(),
            serde_json::json!([{ "operation": 1, "outcome": { "finished": "applied" } }])
        );
    }

    #[test]
    fn a_reader_and_the_writer_exclude_each_other() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture.jsonl");
        std::fs::write(&path, b"").unwrap();
        let reader = File::open(&path).unwrap();
        reader.try_lock_shared().unwrap();
        assert!(matches!(
            LockedJournal::open(&path),
            Err(HistoryFault::Busy)
        ));
        drop(reader);
        let writer = LockedJournal::open(&path).unwrap();
        assert!(matches!(read(&path), Err(HistoryFault::Busy)));
        assert!(matches!(
            LockedJournal::open(&path),
            Err(HistoryFault::Busy)
        ));
        drop(writer);
        assert!(read(&path).is_ok());
    }

    #[test]
    fn a_corrupt_tail_refuses_the_writer_and_keeps_its_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture.jsonl");
        let mut torn = bytes(&[prepared(&apply(1, CONTROL, absent(), dword(1)))]);
        torn.extend_from_slice(b"{\"schema\":");
        std::fs::write(&path, &torn).unwrap();
        assert!(matches!(
            LockedJournal::open(&path),
            Err(HistoryFault::Invalid(_))
        ));
        assert_eq!(std::fs::read(&path).unwrap(), torn);
    }

    /// The writer runs a line through the reader's fold before writing it,
    /// so a line no reader would accept never reaches the file.
    #[test]
    fn a_line_the_history_would_refuse_is_never_written() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture.jsonl");
        let first = apply(1, CONTROL, absent(), dword(1));
        let second = apply(2, CONTROL, absent(), dword(1));
        let mut journal = LockedJournal::open(&path).unwrap();
        journal.append_durable(&prepared(&first)).unwrap();
        assert!(matches!(
            journal.append_durable(&prepared(&second)),
            Err(HistoryFault::Invalid(_))
        ));
        drop(journal);
        assert_eq!(std::fs::read(&path).unwrap(), bytes(&[prepared(&first)]));
    }
}
