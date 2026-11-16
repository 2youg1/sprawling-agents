// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use kernel::{Address, AxError, ReadVerdict};

use super::document::{ConfidentialName, ReaderName, Reason};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidential {
    Withheld,
    Included,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reader {
    Person(Confidential),

    Resident(Address),
}

impl Reader {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Sight {
    Open,

    Closed(Address, Reason),
}

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

    pub(super) fn sight(&mut self, buildings: &BTreeSet<Address>) -> Sight {
        for building in buildings {
            if let Some(reason) = self.closes(building) {
                return Sight::Closed(building.clone(), reason);
            }
        }
        Sight::Open
    }

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

    fn confidential(&self, target: &Address) -> Result<bool, AxError> {
        let holder = city::Building::of(target)?;
        self.rules
            .load(holder.addr())
            .map(|rules| rules.policy().confidential)
    }
}
