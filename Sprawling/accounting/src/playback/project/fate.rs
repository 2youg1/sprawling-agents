// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use kernel::layout::CityLayout;
use kernel::{Address, AxError, EventKind, EventRecord, Seq};
use storage::LedgerIndex;

use super::super::document::{Event, Reason};
use super::super::links::{Links, Visibility};
use super::super::reader::Sight;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::playback) enum Scanning {
    #[cfg(test)]
    Full,

    Lazy,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Fate {
    Shown,
    Closed(Address, Reason),
    Credential,
    Deferred,
}

impl Fate {
    pub(super) fn visibility(&self) -> Visibility {
        match self {
            Fate::Shown => Visibility::Visible,
            Fate::Closed(..) | Fate::Credential => Visibility::Hidden,
            Fate::Deferred => Visibility::Deferred,
        }
    }
}

pub(super) struct Scanner {
    scanning: Scanning,
    scans: u64,

    ledger: PathBuf,
}

impl Scanner {
    pub(super) fn new(scanning: Scanning, city_root: &Path) -> Scanner {
        Scanner {
            scanning,
            scans: 0,
            ledger: CityLayout::new(city_root).ledger(),
        }
    }

    pub(super) fn fate(&mut self, sight: Sight, raw: &[u8], now: bool) -> Fate {
        match sight {
            Sight::Closed(building, reason) => Fate::Closed(building, reason),
            Sight::Open if now && self.carries_credential(raw) => Fate::Credential,
            Sight::Open if now => Fate::Shown,
            Sight::Open => Fate::Deferred,
        }
    }

    pub(super) fn scans_now(&self, record: &EventRecord, in_range: bool) -> bool {
        match self.scanning {
            #[cfg(test)]
            Scanning::Full => true,
            Scanning::Lazy => {
                in_range
                    || matches!(
                        record.kind(),
                        EventKind::RunStarted | EventKind::RunForked | EventKind::ToolCalled
                    )
            }
        }
    }

    pub(super) fn settle(
        &mut self,
        links: &mut Links,
        held: &BTreeMap<Seq, Event>,
        index: &LedgerIndex,
    ) -> Result<(), AxError> {
        let mut reader = index.reader(&self.ledger);
        links.resolve(|seq| match held.get(&seq) {
            Some(event) => Ok(self.carries_credential(event.line.as_bytes())),
            None => reader
                .line_at(seq)
                .map(|raw| self.carries_credential(&raw))
                .map_err(storage::StorageError::into_ax),
        })
    }

    pub(super) fn scans(&self) -> u64 {
        self.scans
    }

    fn carries_credential(&mut self, raw: &[u8]) -> bool {
        self.scans = self.scans.saturating_add(1);
        !kernel::secret::scan(raw).is_empty()
    }
}
