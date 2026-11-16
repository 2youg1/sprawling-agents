// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use kernel::layout::CityLayout;
use kernel::{Address, EventRecord, RESERVED_PREFIX, Seq};

use crate::error::{StorageError, io_err};
use crate::jsonl::complete_lines;
use crate::real_fs::RealFs;
use crate::vfs::Vfs;

mod from_ledger;

pub(crate) const SLICE_MAGIC: &str = "slices v1";

pub struct Sessions {
    layout: CityLayout,
    ledger: PathBuf,
    vfs: RealFs,

    open: BTreeMap<PathBuf, Open>,

    city: Option<Address>,

    first_seen: Option<BTreeMap<Address, Seq>>,
}

struct Open {
    addr: Address,

    last: Option<Seq>,
}

impl Sessions {

    pub(crate) fn for_ledger(dir: &Path) -> Option<Sessions> {
        let layout = CityLayout::of_ledger(dir)?;
        let city = layout.city_address();
        Some(Sessions {
            layout,
            ledger: dir.to_path_buf(),
            vfs: RealFs::new(),
            open: BTreeMap::new(),
            city,
            first_seen: None,
        })
    }

    pub fn absorb(&mut self, record: &EventRecord) -> Result<(), StorageError> {
        let Some(addr) = record.addr() else {
            return Ok(());
        };
        if addr.is_reserved() {
            return Ok(());
        }
        if self.city.as_ref().is_some_and(|city| city == addr) {
            return Ok(());
        }
        if let Some(first_seen) = self.first_seen.as_mut() {
            first_seen.entry(addr.clone()).or_insert(record.seq());
        }
        let path = session_slice(&self.layout, addr);
        if !self.open.contains_key(&path) {
            let open = self.attach(addr, &path, record)?;
            self.open.insert(path.clone(), open);
        }
        let up_to_date = self
            .open
            .get(&path)
            .and_then(|open| open.last)
            .is_some_and(|last| last >= record.seq());
        if up_to_date {
            return Ok(());
        }
        let mut line = record
            .canonical_line()
            .map_err(|source| StorageError::Draft { source })?;
        line.push(b'\n');
        self.vfs
            .append(&path, &line)
            .map_err(io_err("append a session slice", &path))?;
        let open = self.open.get_mut(&path).ok_or_else(|| vanished(&path))?;
        open.last = Some(record.seq());
        Ok(())
    }

    fn attach(
        &mut self,
        addr: &Address,
        path: &Path,
        record: &EventRecord,
    ) -> Result<Open, StorageError> {
        let tail = record.seq();
        if !self.vfs.exists(path) {
            return match self.first_seq_of(addr, tail)? {
                Some(first) if first < tail => self.lay_down(addr, path, tail),
                Some(_) | None => {
                    let line = record
                        .canonical_line()
                        .map_err(|source| StorageError::Draft { source })?;
                    self.write_slice(addr, path, &[(tail, line)])
                }
            };
        }
        let bytes = self
            .vfs
            .read(path)
            .map_err(io_err("read a session slice", path))?;
        let (lines, consumed) = complete_lines(&bytes);
        if consumed < bytes.len() {

            let len = u64::try_from(consumed).unwrap_or(u64::MAX);
            self.vfs
                .truncate(path, len)
                .map_err(io_err("trim a torn session slice", path))?;
        }
        match resume(addr, &lines) {
            Some(open) => self.catch_up(open, path, tail),
            None => self.lay_down(addr, path, tail),
        }
    }

    fn catch_up(&mut self, mut open: Open, path: &Path, tail: Seq) -> Result<Open, StorageError> {
        let found = self.segments().lines_of(&open.addr, open.last, tail)?;
        for (seq, line) in &found {
            let mut terminated = Vec::with_capacity(line.len().saturating_add(1));
            terminated.extend_from_slice(line);
            terminated.push(b'\n');
            self.vfs
                .append(path, &terminated)
                .map_err(io_err("catch up a session slice", path))?;
            open.last = Some(*seq);
        }
        if !found.is_empty() {
            self.vfs
                .sync_data(path)
                .map_err(io_err("flush a session slice", path))?;
        }
        Ok(open)
    }

    fn first_seq_of(&mut self, addr: &Address, tail: Seq) -> Result<Option<Seq>, StorageError> {
        if self.first_seen.is_none() {
            let mut folded = self.segments().first_seen()?;
            folded.entry(addr.clone()).or_insert(tail);
            self.first_seen = Some(folded);
        }
        Ok(self
            .first_seen
            .as_ref()
            .and_then(|folded| folded.get(addr))
            .copied())
    }

    fn segments(&self) -> from_ledger::Segments<'_> {
        from_ledger::Segments {
            vfs: &self.vfs,
            ledger: &self.ledger,
        }
    }

    fn lay_down(&mut self, addr: &Address, path: &Path, tail: Seq) -> Result<Open, StorageError> {
        let found = self.segments().lines_of(addr, None, tail)?;
        self.write_slice(addr, path, &found)
    }

    fn write_slice(
        &mut self,
        addr: &Address,
        path: &Path,
        found: &[(Seq, Vec<u8>)],
    ) -> Result<Open, StorageError> {
        let Some((first_seq, _)) = found.first() else {

            return Ok(Open {
                addr: addr.clone(),
                last: None,
            });
        };
        let mut body = format!("{SLICE_MAGIC} {}\n", first_seq.value()).into_bytes();
        let mut last = *first_seq;
        for (seq, line) in found {
            body.extend_from_slice(line);
            body.push(b'\n');
            last = *seq;
        }
        let Some(dir) = path.parent() else {
            return Err(no_parent(path));
        };
        self.vfs
            .create_dir_all(dir)
            .map_err(io_err("create the sessions directory", dir))?;
        if self.vfs.exists(path) {
            self.vfs
                .remove_file(path)
                .map_err(io_err("replace a session slice", path))?;
        }
        self.vfs
            .append(path, &body)
            .map_err(io_err("lay down a session slice", path))?;
        self.vfs
            .sync_data(path)
            .map_err(io_err("flush a session slice", path))?;
        Ok(Open {
            addr: addr.clone(),
            last: Some(last),
        })
    }
}

fn resume(addr: &Address, lines: &[&[u8]]) -> Option<Open> {
    let header = lines.first()?;
    let header_seq = parse_header(header)?;
    let mut last: Option<Seq> = None;
    for (index, line) in lines.iter().enumerate().skip(1) {
        let record = EventRecord::parse_line(line).ok()?;
        if record.addr() != Some(addr) {
            return None;
        }
        if index == 1 && record.seq() != header_seq {
            return None;
        }
        if last.is_some_and(|seen| record.seq() <= seen) {
            return None;
        }
        last = Some(record.seq());
    }

    let last = last?;
    Some(Open {
        addr: addr.clone(),
        last: Some(last),
    })
}

fn parse_header(line: &[u8]) -> Option<Seq> {
    let text = std::str::from_utf8(line).ok()?;
    let digits = text.strip_prefix(SLICE_MAGIC)?.strip_prefix(' ')?;
    if digits.contains(' ') {
        return None;
    }
    digits.parse::<u64>().ok().map(Seq::new)
}

const SESSIONS_DIR: &str = "sessions";

fn session_slice(layout: &CityLayout, room: &Address) -> PathBuf {
    let raw = room.as_str();
    let (building, under) = raw.split_once('/').unwrap_or((raw, ""));
    let name = if under.is_empty() { building } else { under };
    let mut path = layout.root().join(building);
    path.push(RESERVED_PREFIX);
    path.push(SESSIONS_DIR);
    path.push(format!("{name}.jsonl"));
    path
}

pub(crate) fn is_session_projection(relative: &Path) -> bool {
    let mut held = false;
    for component in relative.components() {
        let Some(name) = component.as_os_str().to_str() else {
            held = false;
            continue;
        };
        if name.eq_ignore_ascii_case(SESSIONS_DIR) {
            return held;
        }
        held = name.eq_ignore_ascii_case(RESERVED_PREFIX);
    }
    false
}

fn no_parent(path: &Path) -> StorageError {
    StorageError::Io {
        op: "lay a session slice",
        path: path.to_path_buf(),
        source: io::Error::other("the slice path has no parent directory"),
    }
}

fn vanished(path: &Path) -> StorageError {
    StorageError::Io {
        op: "continue a session slice",
        path: path.to_path_buf(),
        source: io::Error::other("the slice state vanished after it was read"),
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
mod tests;
