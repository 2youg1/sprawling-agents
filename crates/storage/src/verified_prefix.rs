// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a strict walk of the chain proved, one record per segment, so the
//! next proof reads and hashes those bytes instead of checking them line
//! by line again (storage-SPEC 8-30).
//!
//! A projection: a record that is missing, damaged, of another rule
//! version, or out of step with the chain state entering its segment is
//! not used, and that segment is checked line by line. That reusing a
//! record gives the strict verdict is `cachedVerifyIsStrict` in
//! `crates/storage/spec/Snapshot.lean`.

use std::io;
use std::path::{Path, PathBuf};

use kernel::{B3Hash, Seq};

use crate::error::StorageError;
use crate::jsonl::{JsonlLedger, LineCheck};
use crate::real_fs::RealFs;
use crate::vfs::Vfs;

/// Changed whenever the per-line check changes what it accepts within one
/// version of this binary; a new version discards every record anyway,
/// because [`line_check_version`] hashes the version in with this. The
/// suffix is the digest of a fixed fixture's verdicts, which
/// `chain_audit::tests` holds, so the rule cannot move alone.
pub(crate) const LINE_CHECK_RULES: &str = "line-check-40e703f48b222ca7";

const MAGIC: &[u8; 8] = b"SPRPRF01";
const SUFFIX: &str = ".proof";
const STAGED: &str = ".staged";

/// The rule version a record is written and accepted under.
pub fn line_check_version() -> u32 {
    let rules = [env!("CARGO_PKG_VERSION"), LINE_CHECK_RULES].join("\n");
    let [a, b, c, d, ..] = *B3Hash::digest(rules.as_bytes()).as_bytes();
    u32::from_le_bytes([a, b, c, d])
}

/// Where a city's records live, and whether this holder may write them.
#[derive(Debug, Clone)]
pub struct ProofRecords {
    dir: PathBuf,
    may: May,
}

/// Only a handle that holds the writer lock writes records, so a
/// read-only command never touches the disk (storage-SPEC 8-30).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum May {
    Read,
    ReadAndWrite,
}

impl ProofRecords {
    /// Records read, never written: `ask`, `replay`, and any holder that
    /// does not hold the writer lock.
    pub fn read_only(dir: &Path) -> ProofRecords {
        ProofRecords {
            dir: dir.to_path_buf(),
            may: May::Read,
        }
    }

    /// The record of the segment named `segment`, when there is one this
    /// build can read. Absent, unreadable and damaged all answer `None`:
    /// the segment is then checked line by line, which is the answer this
    /// projection gives to every doubt.
    pub(crate) fn read(&self, segment: &str) -> Option<SegmentRecord> {
        let path = self.path_of(segment);
        match RealFs::new().read(&path) {
            Ok(bytes) => SegmentRecord::decode(&bytes),
            Err(_unreadable) => None,
        }
    }

    /// Writes `record` for `segment`, replacing the one there; nothing
    /// for a read-only holder.
    ///
    /// # Errors
    /// `StorageError::Snapshot` naming the step that failed.
    pub(crate) fn keep(&self, segment: &str, record: &SegmentRecord) -> Result<(), StorageError> {
        match self.may {
            May::Read => Ok(()),
            May::ReadAndWrite => self.write(segment, record),
        }
    }

    fn write(&self, segment: &str, record: &SegmentRecord) -> Result<(), StorageError> {
        let mut vfs = RealFs::new();
        let file = self.path_of(segment);
        let staged = self.dir.join(format!("{segment}{SUFFIX}{STAGED}"));
        vfs.create_dir_all(&self.dir)
            .map_err(refused("create the proof records dir", &self.dir))?;
        if vfs.exists(&staged) {
            vfs.remove_file(&staged)
                .map_err(refused("remove a staged proof record", &staged))?;
        }
        vfs.append(&staged, &record.encode())
            .map_err(refused("write a staged proof record", &staged))?;
        vfs.sync_data(&staged)
            .map_err(refused("sync a staged proof record", &staged))?;
        vfs.rename(&staged, &file)
            .map_err(refused("rename a staged proof record", &file))?;
        vfs.sync_dir(&self.dir)
            .map_err(refused("sync the proof records dir", &self.dir))
    }

    fn path_of(&self, segment: &str) -> PathBuf {
        self.dir.join(format!("{segment}{SUFFIX}"))
    }
}

impl JsonlLedger {
    /// The records of this ledger's proofs, kept in `dir`: writable when
    /// this handle holds the ledger's writer lock, read-only otherwise.
    pub fn proof_records(&self, dir: &Path) -> ProofRecords {
        ProofRecords {
            dir: dir.to_path_buf(),
            may: match self.lock {
                Some(_) => May::ReadAndWrite,
                None => May::Read,
            },
        }
    }
}

/// What a strict walk proved about the first `len` bytes of one segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SegmentRecord {
    pub(crate) version: u32,
    /// The length of the proved prefix: always the end of a complete line.
    pub(crate) len: u64,
    pub(crate) lines: u64,
    pub(crate) entry: LineCheck,
    pub(crate) exit: LineCheck,
    pub(crate) digest: B3Hash,
}

impl SegmentRecord {
    /// `MAGIC | version u32 | len u64 | lines u64 | entry | exit | digest |
    /// blake3 of everything before it`, a chain state being its `prev`
    /// hash and the seq it expects, little-endian.
    fn encode(&self) -> Vec<u8> {
        let body = [
            MAGIC.as_slice(),
            &self.version.to_le_bytes(),
            &self.len.to_le_bytes(),
            &self.lines.to_le_bytes(),
            self.entry.prev().as_bytes(),
            &self.entry.expected().value().to_le_bytes(),
            self.exit.prev().as_bytes(),
            &self.exit.expected().value().to_le_bytes(),
            self.digest.as_bytes(),
        ]
        .concat();
        let seal = B3Hash::digest(&body);
        [body.as_slice(), seal.as_bytes()].concat()
    }

    /// The record `encode` wrote; `None` for bytes that are not one.
    fn decode(bytes: &[u8]) -> Option<SegmentRecord> {
        let (body, seal) = bytes.split_last_chunk::<32>()?;
        if B3Hash::digest(body).as_bytes() != seal {
            return None;
        }
        let (magic, rest) = body.split_first_chunk::<8>()?;
        if magic != MAGIC {
            return None;
        }
        let (version, rest) = rest.split_first_chunk::<4>()?;
        let (len, rest) = rest.split_first_chunk::<8>()?;
        let (lines, rest) = rest.split_first_chunk::<8>()?;
        let (entry, rest) = chain_state(rest)?;
        let (exit, rest) = chain_state(rest)?;
        let digest = <[u8; 32]>::try_from(rest).ok()?;
        Some(SegmentRecord {
            version: u32::from_le_bytes(*version),
            len: u64::from_le_bytes(*len),
            lines: u64::from_le_bytes(*lines),
            entry,
            exit,
            digest: B3Hash::from_bytes(digest),
        })
    }
}

fn chain_state(bytes: &[u8]) -> Option<(LineCheck, &[u8])> {
    let (prev, rest) = bytes.split_first_chunk::<32>()?;
    let (expected, rest) = rest.split_first_chunk::<8>()?;
    Some((
        LineCheck::after(
            B3Hash::from_bytes(*prev),
            Seq::new(u64::from_le_bytes(*expected)),
        ),
        rest,
    ))
}

/// The constructor of a record's I/O failure. A record is a projection
/// like a snapshot, so it shares the snapshot's advice.
fn refused(op: &'static str, path: &Path) -> impl FnOnce(io::Error) -> StorageError {
    let path = path.to_path_buf();
    move |source| StorageError::Snapshot { op, path, source }
}
