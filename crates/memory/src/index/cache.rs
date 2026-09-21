// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Index cache: believed on stamp match, rebuilt on any doubt.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use kernel::{B3Hash, RunId, Seq};

use crate::error::{MemoryError, io_err};
use crate::jsonl::is_segment;
use crate::vfs::Vfs;

/// What the directory looks like right now: total bytes across segments
/// plus a digest of the segment names and sizes. Coarse on purpose — a
/// side artifact should rebuild too often rather than be trusted once
/// too many.
pub(crate) const CACHE_NAME: &str = "index.cache";
/// Bumped to v2 when the cache started carrying each line's run. An
/// older cache fails this comparison and rebuilds in silence, which is
/// this module's standing answer to any doubt — so no migration code
/// exists, and none is needed.
pub(crate) const CACHE_MAGIC: &str = "idx v2";
/// What a cache row writes where the run belongs when the line named
/// none. Not a valid uuid, so it can never be read back as one.
pub(crate) const NO_RUN: &str = "-";

pub(crate) struct Stamp {
    pub(crate) bytes: u64,
    pub(crate) digest: String,
}

/// Everything the index knows, with no way to read a disk.
///
/// Held apart from [`crate::index::LedgerIndex`] because a rebuild
/// produces exactly this and nothing else: the seam the index reaches
/// disk through belongs to the index, not to each map it folds.
pub(crate) struct Folded {
    pub(crate) entries: BTreeMap<Seq, (String, u64)>,
    pub(crate) runs: BTreeMap<RunId, BTreeSet<Seq>>,
    pub(crate) scanned: BTreeMap<String, u64>,
}

impl Folded {
    pub(crate) fn empty() -> Folded {
        Folded {
            entries: BTreeMap::new(),
            runs: BTreeMap::new(),
            scanned: BTreeMap::new(),
        }
    }

    /// Indexes the complete lines of `tail`, which begins at `from` in
    /// the segment. Returns how many bytes those complete lines took.
    pub(crate) fn fold_segment(&mut self, name: &str, from: u64, tail: &[u8]) -> u64 {
        let mut offset = from;
        let mut complete_bytes = 0u64;
        for line in tail.split_inclusive(|b| *b == b'\n') {
            if line.last().copied() != Some(b'\n') {
                break;
            }
            let body = line.get(..line.len().saturating_sub(1)).unwrap_or(line);
            if !body.is_empty() {
                self.insert_line(name, offset, body);
            }
            let len = u64::try_from(line.len()).unwrap_or(0);
            offset = offset.saturating_add(len);
            complete_bytes = complete_bytes.saturating_add(len);
        }
        complete_bytes
    }

    /// What indexing one line means, in one place: where it sits, and
    /// whose it is. A line that does not parse is skipped rather than
    /// reported — the caller is looking at a ledger that may be damaged,
    /// and an index is how such a ledger gets repaired.
    pub(crate) fn insert_line(&mut self, name: &str, offset: u64, body: &[u8]) {
        let Some(located) = locate(body) else {
            return;
        };
        self.entries.insert(located.seq, (name.to_owned(), offset));
        if let Some(run) = located.run {
            self.runs.entry(run).or_default().insert(located.seq);
        }
    }
}

pub(crate) fn segment_names(vfs: &dyn Vfs, dir: &Path) -> Result<Vec<String>, MemoryError> {
    let mut names = Vec::new();
    // `Vfs::list` answers files only, already sorted: zero-padded names
    // sort lexically the way they sort numerically, so the order is the
    // ledger's own rather than the filesystem's.
    for path in vfs.list(dir).map_err(io_err("list ledger dir", dir))? {
        if !is_segment(&path) {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            names.push(name.to_owned());
        }
    }
    Ok(names)
}

pub(crate) fn directory_stamp(vfs: &dyn Vfs, dir: &Path) -> Result<Stamp, MemoryError> {
    let mut total: u64 = 0;
    let mut material = String::new();
    for name in segment_names(vfs, dir)? {
        let path = dir.join(&name);
        let size = vfs.size(&path).map_err(io_err("stat segment", &path))?;
        total = total.saturating_add(size);
        material.push_str(&format!("{name}:{size}\n"));
    }
    let digest = B3Hash::digest(material.as_bytes()).to_string();
    let short = digest.get(..16).unwrap_or(&digest).to_owned();
    Ok(Stamp {
        bytes: total,
        digest: short,
    })
}

/// How many bytes of each segment exist right now.
///
/// A cache is believed only when the directory's byte count matches its
/// stamp, so at that moment "already indexed" and "size on disk" are the
/// same number - this is exact rather than an approximation.
pub(crate) fn sizes_now(vfs: &dyn Vfs, dir: &Path) -> BTreeMap<String, u64> {
    let mut sizes = BTreeMap::new();
    let Ok(names) = segment_names(vfs, dir) else {
        return sizes;
    };
    for name in names {
        if let Ok(size) = vfs.size(&dir.join(&name)) {
            sizes.insert(name, size);
        }
    }
    sizes
}

pub(crate) fn load_cache(vfs: &dyn Vfs, dir: &Path, stamp: &Stamp) -> Option<Folded> {
    let bytes = vfs.read(&dir.join(CACHE_NAME)).ok()?;
    let raw = String::from_utf8(bytes).ok()?;
    let mut lines = raw.lines();
    let header = lines.next()?;
    let expected = format!("{CACHE_MAGIC} {} {}", stamp.bytes, stamp.digest);
    if header != expected {
        return None;
    }
    let mut entries = BTreeMap::new();
    let mut runs: BTreeMap<RunId, BTreeSet<Seq>> = BTreeMap::new();
    for line in lines {
        let mut parts = line.split(' ');
        let seq = Seq::new(parts.next()?.parse::<u64>().ok()?);
        let segment = parts.next()?.to_owned();
        let offset = parts.next()?.parse::<u64>().ok()?;
        // A row that names no run is how the writer spells a line whose
        // own run was unreadable; anything else that will not parse as a
        // run id makes the whole cache suspect, so the caller rebuilds.
        let owner = match parts.next()? {
            NO_RUN => None,
            raw => Some(RunId::parse(raw).ok()?),
        };
        if parts.next().is_some() {
            return None;
        }
        entries.insert(seq, (segment, offset));
        if let Some(run) = owner {
            runs.entry(run).or_default().insert(seq);
        }
    }
    Some(Folded {
        entries,
        runs,
        scanned: sizes_now(vfs, dir),
    })
}

pub(crate) fn rebuild(vfs: &dyn Vfs, dir: &Path) -> Result<Folded, MemoryError> {
    let mut folded = Folded::empty();
    for name in segment_names(vfs, dir)? {
        let path = dir.join(&name);
        let bytes = vfs.read(&path).map_err(io_err("read segment", &path))?;
        let mut offset: u64 = 0;
        for line in bytes.split_inclusive(|b| *b == b'\n') {
            let complete = line.last().copied() == Some(b'\n');
            let body = if complete {
                line.get(..line.len().saturating_sub(1)).unwrap_or(line)
            } else {
                line
            };
            // A torn tail carries no seq we can trust; it is skipped, and
            // the next append overwrites it (jsonl owns that repair).
            if complete && !body.is_empty() {
                folded.insert_line(&name, offset, body);
            }
            if complete {
                folded.scanned.insert(
                    name.clone(),
                    offset.saturating_add(u64::try_from(line.len()).unwrap_or(0)),
                );
            }
            offset = offset.saturating_add(u64::try_from(line.len()).unwrap_or(0));
        }
        folded.scanned.entry(name).or_insert(0);
    }
    Ok(folded)
}

/// Where a line sits and whose it is.
pub(crate) struct Located {
    pub(crate) seq: Seq,
    /// `None` when the line names no run that reads back as one. The
    /// line is still indexed by seq: an index over a damaged ledger is
    /// exactly what a repair path needs, and a line dropped here would
    /// be invisible to every reader.
    pub(crate) run: Option<RunId>,
}

/// Reads two fields off one parse. Indexing must not depend on the
/// record parsing cleanly as an `EventRecord`, so this asks the raw
/// document rather than the typed one.
pub(crate) fn locate(line: &[u8]) -> Option<Located> {
    let value: serde_json::Value = serde_json::from_slice(line).ok()?;
    let seq = Seq::new(value.get("seq")?.as_u64()?);
    let run = value
        .get("run")
        .and_then(serde_json::Value::as_str)
        .and_then(|raw| RunId::parse(raw).ok());
    Some(Located { seq, run })
}
