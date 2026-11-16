// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What becomes of one line, and which lines the credential scan reads
//! (`crates/accounting/spec/Playback/Project.lean`, last section, D47).
//!
//! A selected line, and a line whose fate a table reads without pairing
//! it, is scanned as it is folded. Any other line outside the selection
//! whose buildings all read `Open` is deferred: [`Scanner::settle`]
//! scans it once the fold has ended, only if it is the far end of a
//! pair with a selected line the reader sees. The model proves the two
//! scans hand every table the same fates.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use kernel::layout::CityLayout;
use kernel::{Address, AxError, EventKind, EventRecord, Seq};
use storage::LedgerIndex;

use super::super::document::{Event, Reason};
use super::super::links::{Links, Visibility};
use super::super::reader::Sight;

/// Which lines the projection runs the credential scan on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::playback) enum Scanning {
    /// Every line whose buildings all read `Open`: the reference the lazy
    /// scan is compared against.
    #[cfg(test)]
    Full,
    /// Only the lines whose fate some table reads.
    Lazy,
}

/// What becomes of one line: shown, closed by a building, withheld
/// because the credential scan matched it, or, outside the selection,
/// open until a table reads it.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Fate {
    Shown,
    Closed(Address, Reason),
    Credential,
    Deferred,
}

impl Fate {
    /// What the links know of the line while they pair it.
    pub(super) fn visibility(&self) -> Visibility {
        match self {
            Fate::Shown => Visibility::Visible,
            Fate::Closed(..) | Fate::Credential => Visibility::Hidden,
            Fate::Deferred => Visibility::Deferred,
        }
    }
}

/// The credential scan of one projection, and how many lines it read.
pub(super) struct Scanner {
    scanning: Scanning,
    scans: u64,
    /// Where a deferred line's bytes are read back from by offset.
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

    /// The fate of the line `raw`, by what the reader may see of its
    /// buildings and, when `now` says the scan reads it as it is
    /// folded, by its bytes.
    pub(super) fn fate(&mut self, sight: Sight, raw: &[u8], now: bool) -> Fate {
        match sight {
            Sight::Closed(building, reason) => Fate::Closed(building, reason),
            Sight::Open if now && self.carries_credential(raw) => Fate::Credential,
            Sight::Open if now => Fate::Shown,
            Sight::Open => Fate::Deferred,
        }
    }

    /// Whether a line is scanned as it is folded: it is selected, or a
    /// table reads its fate without pairing it to a selected line —
    /// `remember()` the `run_started` and `tool_called` lines, and
    /// `Links::related` the opening line of a run another run points at.
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

    /// Scans the deferred far ends of every selected pair: from the copy
    /// `held` keeps for the bundle's context, or read back by offset
    /// through `index`.
    ///
    /// # Errors
    /// A line the index locates and its segment cannot give back.
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

    /// How many lines this projection has scanned.
    pub(super) fn scans(&self) -> u64 {
        self.scans
    }

    fn carries_credential(&mut self, raw: &[u8]) -> bool {
        self.scans = self.scans.saturating_add(1);
        !kernel::secret::scan(raw).is_empty()
    }
}
