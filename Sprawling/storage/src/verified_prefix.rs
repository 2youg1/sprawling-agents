// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io;
use std::path::{Path, PathBuf};

use kernel::{B3Hash, Seq};

use crate::chain_audit::ProofCount;
use crate::error::StorageError;
use crate::jsonl::{JsonlLedger, LineCheck};
use crate::real_fs::RealFs;
use crate::vfs::Vfs;

pub(crate) const LINE_CHECK_RULES: &str = "line-check-40e703f48b222ca7";

const MAGIC: &[u8; 8] = b"SPRPRF01";
const SUFFIX: &str = ".proof";
const STAGED: &str = ".staged";

pub fn line_check_version() -> u32 {
    let rules = [env!("CARGO_PKG_VERSION"), LINE_CHECK_RULES].join("\n");
    let [a, b, c, d, ..] = *B3Hash::digest(rules.as_bytes()).as_bytes();
    u32::from_le_bytes([a, b, c, d])
}

#[derive(Debug, Clone)]
pub struct ProofRecords {
    dir: PathBuf,
    may: May,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum May {
    Read,
    ReadAndWrite,
}

impl ProofRecords {

    pub fn read_only(dir: &Path) -> ProofRecords {
        ProofRecords {
            dir: dir.to_path_buf(),
            may: May::Read,
        }
    }

    pub(crate) fn read(&self, segment: &str) -> Option<SegmentRecord> {
        let path = self.path_of(segment);
        match RealFs::new().read(&path) {
            Ok(bytes) => SegmentRecord::decode(&bytes),
            Err(_unreadable) => None,
        }
    }

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

pub(crate) struct TailStart {
    pub(crate) from: usize,
    pub(crate) lines: u64,
    pub(crate) check: LineCheck,
    pub(crate) counted: ProofCount,
}

impl TailStart {

    pub(crate) fn strict(entry: LineCheck) -> TailStart {
        TailStart {
            from: 0,
            lines: 0,
            check: entry,
            counted: ProofCount::default(),
        }
    }
}

impl ProofRecords {

    pub(crate) fn tail_start(
        &self,
        vfs: &dyn Vfs,
        path: &Path,
        entry: LineCheck,
    ) -> io::Result<TailStart> {
        let mut start = TailStart::strict(entry);
        let Some(record) = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| self.read(name))
        else {
            return Ok(start);
        };
        let Some(length) = record.prefix_length(vfs.size(path)?, line_check_version()) else {
            return Ok(start);
        };
        if record.entry != entry {
            return Ok(start);
        }
        let mut hasher = blake3::Hasher::new();
        let mut offset = 0u64;
        while offset < length {
            let chunk = vfs.read_at(
                path,
                offset,
                crate::jsonl::SCAN_WINDOW_BYTES.min(length.saturating_sub(offset)),
            )?;
            if chunk.is_empty() {
                return Ok(start);
            }
            #[cfg(test)]
            crate::jsonl::measure_scan(chunk.capacity(), 0);
            hasher.update(&chunk);
            let read = u64::try_from(chunk.len()).map_err(io::Error::other)?;
            offset = offset
                .checked_add(read)
                .ok_or_else(|| io::Error::other("prefix offset overflow"))?;
            start.counted.bytes_read = start.counted.bytes_read.saturating_add(read);
            start.counted.bytes_hashed = start.counted.bytes_hashed.saturating_add(read);
        }
        if let Some((lines, exit)) = record.stands(entry, &hasher) {
            start.from = usize::try_from(length).map_err(io::Error::other)?;
            start.lines = lines;
            start.check = exit;
            start.counted.segments_by_digest = 1;
        }
        Ok(start)
    }
}

impl JsonlLedger {

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SegmentRecord {
    pub(crate) version: u32,

    pub(crate) len: u64,
    pub(crate) lines: u64,
    pub(crate) entry: LineCheck,
    pub(crate) exit: LineCheck,
    pub(crate) digest: B3Hash,
}

impl SegmentRecord {

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

impl SegmentRecord {

    pub(crate) fn prefix_length(&self, total: u64, version: u32) -> Option<u64> {
        (self.version == version && self.len <= total).then_some(self.len)
    }

    pub(crate) fn prefix<'a>(&self, bytes: &'a [u8], version: u32) -> Option<&'a [u8]> {
        self.prefix_length(u64::try_from(bytes.len()).ok()?, version)
            .and_then(|len| usize::try_from(len).ok())
            .and_then(|len| bytes.get(..len))
    }

    pub(crate) fn stands(
        &self,
        entry: LineCheck,
        hashed: &blake3::Hasher,
    ) -> Option<(u64, LineCheck)> {
        (self.entry == entry && B3Hash::from_bytes(*hashed.finalize().as_bytes()) == self.digest)
            .then_some((self.lines, self.exit))
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

fn refused(op: &'static str, path: &Path) -> impl FnOnce(io::Error) -> StorageError {
    let path = path.to_path_buf();
    move |source| StorageError::Snapshot { op, path, source }
}
