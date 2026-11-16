// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::Effect;
use crate::Address;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Writes {

    Nothing,

    Paths(Vec<Address>),

    Domain,
}

impl Writes {

    pub fn of(effect: &Effect) -> Self {
        match effect {
            Effect::Read => Self::Nothing,
            Effect::Write { .. }
            | Effect::Egress
            | Effect::Connector { .. }
            | Effect::Spawn
            | Effect::Govern
            | Effect::AttachUserBrowser { .. }
            | Effect::Spend => Self::Domain,
        }
    }

    #[must_use]
    pub fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::Domain, _) | (_, Self::Domain) => Self::Domain,
            (Self::Nothing, only) | (only, Self::Nothing) => only,
            (Self::Paths(mut these), Self::Paths(those)) => {
                for path in those {
                    if !these.contains(&path) {
                        these.push(path);
                    }
                }
                Self::Paths(these)
            }
        }
    }
}
