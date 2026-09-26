// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a call that ran may have written to the city's tree, as its tool
//! can tell (kernel-SPEC section 8-23).

use super::Effect;
use crate::Address;

/// What one call, or every call since the last fence, may have written.
///
/// A fence stages what this names (runtime-SPEC section 8-45), so an
/// answer narrower than the truth leaves a write no commit carries. Only
/// a tool that knows every file it wrote answers `Paths`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Writes {
    /// Nothing in the tree.
    Nothing,
    /// Exactly these files.
    Paths(Vec<Address>),
    /// Somewhere in the run's write domain, which the fence then walks.
    Domain,
}

impl Writes {
    /// The default answer, read off the declared effect alone: only a
    /// `Read` promises to leave the tree alone, and every other effect -
    /// a command, a desk, a rule change, a spawned worktree - may write
    /// where no argument says.
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

    /// Both answers at once: `Domain` absorbs everything, `Nothing` adds
    /// nothing, and two sets of paths join without duplicates.
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
