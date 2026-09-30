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
