// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::path::Path;

use kernel::{Address, EventRecord, Seq};

use crate::error::{StorageError, io_err};
use crate::jsonl::{complete_lines, segment_first_seq, segment_names};
use crate::vfs::Vfs;

pub(super) struct Segments<'a> {
    pub(super) vfs: &'a dyn Vfs,
    pub(super) ledger: &'a Path,
}

impl Segments<'_> {

    pub(super) fn first_seen(&self) -> Result<BTreeMap<Address, Seq>, StorageError> {
        let (vfs, ledger) = (self.vfs, self.ledger);
        let mut folded = BTreeMap::new();
        for name in segment_names(vfs, ledger)? {
            let path = ledger.join(name);
            let bytes = vfs
                .read(&path)
                .map_err(io_err("read a ledger segment", &path))?;
            let (lines, _) = complete_lines(&bytes);
            for record in lines
                .into_iter()
                .filter_map(|line| EventRecord::parse_line(line).ok())
            {
                if let Some(addr) = record.addr() {
                    folded.entry(addr.clone()).or_insert(record.seq());
                }
            }
        }
        Ok(folded)
    }

    pub(super) fn lines_of(
        &self,
        addr: &Address,
        after: Option<Seq>,
        through: Seq,
    ) -> Result<Vec<(Seq, Vec<u8>)>, StorageError> {
        let (vfs, ledger) = (self.vfs, self.ledger);
        let names = segment_names(vfs, ledger)?;
        let mut found = Vec::new();
        for (index, name) in names.iter().enumerate() {
            if !holds_a_later_record(&names, index, after) {
                continue;
            }
            let path = ledger.join(name);
            let bytes = vfs
                .read(&path)
                .map_err(io_err("read a ledger segment", &path))?;
            let (lines, _) = complete_lines(&bytes);
            for line in lines {
                let Ok(record) = EventRecord::parse_line(line) else {
                    continue;
                };
                if record.addr() != Some(addr) {
                    continue;
                }
                if after.is_some_and(|seen| record.seq() <= seen) {
                    continue;
                }
                if record.seq() > through {
                    continue;
                }
                found.push((record.seq(), line.to_vec()));
            }
        }
        Ok(found)
    }
}

fn holds_a_later_record(names: &[String], index: usize, after: Option<Seq>) -> bool {
    let Some(after) = after else {
        return true;
    };
    match names
        .get(index.saturating_add(1))
        .and_then(|name| segment_first_seq(name))
    {
        Some(next) => next > after,

        None => true,
    }
}
