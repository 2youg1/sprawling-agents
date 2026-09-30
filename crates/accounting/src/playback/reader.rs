// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Who reads a playback bundle, and whether one line is open to them
//! (accounting-SPEC.md 8-12). The property is
//! `crates/accounting/spec/Playback/Project.lean`: a line is visible only
//! when every building it touches is `Open`, and the two other arms of
//! `kernel::ReadVerdict` both close.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use kernel::{Address, AxError, ReadVerdict};

use super::document::{ConfidentialName, ReaderName, Reason};

/// Whether the person's bundle carries confidential buildings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidential {
    Withheld,
    Included,
}

/// Who the bundle is for. The entry that exports or checks says so; a
/// bundle's own account of its reader is never believed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reader {
    /// The person, at the command line.
    Person(Confidential),
    /// A resident of this building, through its tool.
    Resident(Address),
}

impl Reader {
    /// The reader as the bundle's `source` names it.
    pub(super) fn name(&self) -> ReaderName {
        match self {
            Reader::Person(Confidential::Withheld) => {
                ReaderName::Person(ConfidentialName::Withheld)
            }
            Reader::Person(Confidential::Included) => {
                ReaderName::Person(ConfidentialName::Included)
            }
            Reader::Resident(building) => ReaderName::Resident(building.clone()),
        }
    }
}

/// What one line is to the reader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Sight {
    Open,
    /// The first building, in address order, that closed the line.
    Closed(Address, Reason),
}

/// The reader's verdict on each building, asked once per export from
/// the city's rules as they stand now.
pub(super) struct Readership {
    reader: Reader,
    rules: city::RulesCache,
    verdicts: BTreeMap<Address, Option<Reason>>,
}

impl Readership {
    pub(super) fn new(city_root: &Path, reader: Reader) -> Readership {
        Readership {
            reader,
            rules: city::RulesCache::new(city_root),
            verdicts: BTreeMap::new(),
        }
    }

    /// What a line touching `buildings` is to the reader: open only when
    /// every one of them is.
    pub(super) fn sight(&mut self, buildings: &BTreeSet<Address>) -> Sight {
        for building in buildings {
            if let Some(reason) = self.closes(building) {
                return Sight::Closed(building.clone(), reason);
            }
        }
        Sight::Open
    }

    /// Whether the reader is closed out of `building`, and why.
    pub(super) fn closes(&mut self, building: &Address) -> Option<Reason> {
        if let Some(known) = self.verdicts.get(building) {
            return *known;
        }
        let verdict = match &self.reader {
            Reader::Person(Confidential::Included) => ReadVerdict::Open,
            Reader::Person(Confidential::Withheld) => match self.confidential(building) {
                Ok(false) => ReadVerdict::Open,
                Ok(true) => ReadVerdict::Confidential,
                Err(unread) => ReadVerdict::RulesUnreadable(unread),
            },
            Reader::Resident(home) => {
                kernel::address::may_read(home, building, || self.confidential(building))
            }
        };
        let reason = match verdict {
            ReadVerdict::Open => None,
            ReadVerdict::Confidential => Some(Reason::Confidential),
            ReadVerdict::RulesUnreadable(_) => Some(Reason::RulesUnreadable),
        };
        self.verdicts.insert(building.clone(), reason);
        reason
    }

    /// Whether the rules of the building holding `target` say it is
    /// confidential, as the read bound asks it.
    fn confidential(&self, target: &Address) -> Result<bool, AxError> {
        let holder = city::Building::of(target)?;
        self.rules
            .load(holder.addr())
            .map(|rules| rules.policy().confidential)
    }
}
