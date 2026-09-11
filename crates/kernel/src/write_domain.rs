// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! WriteDomain prefix checking and edit-war detection. C17 is enforced twice on purpose: a reserved prefix cannot be
//! *constructed into* a domain, and a reserved target is refused at
//! judgment even so — fail-closed has no single point of failure.

use std::collections::BTreeMap;

use crate::address::Address;
use crate::consts_policy::EDIT_WAR_FREEZE;
use crate::error::{AxCode, AxError};
use crate::event::RunId;

/// A validated set of prefixes.
///
/// The one construction point C17 is enforced at, and the reason
/// [`WriteDomain`] can be an enum with public variants without gaining a
/// second way in: a variant is spellable, but only with a value that has
/// already been checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainPrefixes {
    prefixes: Vec<Address>,
}

impl DomainPrefixes {
    /// C17 at the construction point: any member under the reserved
    /// prefix refuses the whole set. An empty set is legal (a read-only
    /// actor writes nowhere).
    ///
    /// # Errors
    /// `E_INVALID_ARGS` naming the reserved member.
    pub fn new(prefixes: Vec<Address>) -> Result<DomainPrefixes, AxError> {
        if let Some(reserved) = prefixes.iter().find(|p| p.is_reserved()) {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "construct write domain",
                reserved.as_str(),
            )
            .with_recovery(
                "the reserved prefix is outside every write domain (C17); \
                 drop it from the prefix set",
            ));
        }
        Ok(DomainPrefixes { prefixes })
    }

    pub fn iter(&self) -> impl Iterator<Item = &Address> {
        self.prefixes.iter()
    }
}

/// The set of prefixes one actor may write under, and what it may write
/// inside them.
///
/// Exhaustive rather than a set plus a flag: "every file here" and "the
/// Markdown documents here" are two policies, and the second one is what
/// City Hall's residents are given so that a planner which may not build
/// is a type rather than a sentence in a prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteDomain {
    /// Every file under the prefixes.
    Everything(DomainPrefixes),
    /// Markdown documents under the prefixes, and never a plan file:
    /// `Roadmap.md` has one editing entrance, and it is the `plan` tool.
    Documents(DomainPrefixes),
}

impl WriteDomain {
    /// The ordinary domain: every file under these prefixes.
    ///
    /// # Errors
    /// Propagates [`DomainPrefixes::new`].
    pub fn new(prefixes: Vec<Address>) -> Result<WriteDomain, AxError> {
        Ok(WriteDomain::Everything(DomainPrefixes::new(prefixes)?))
    }

    /// The documents-only domain.
    ///
    /// # Errors
    /// Propagates [`DomainPrefixes::new`].
    pub fn documents(prefixes: Vec<Address>) -> Result<WriteDomain, AxError> {
        Ok(WriteDomain::Documents(DomainPrefixes::new(prefixes)?))
    }

    pub fn prefixes(&self) -> impl Iterator<Item = &Address> {
        match self {
            WriteDomain::Everything(set) | WriteDomain::Documents(set) => set.iter(),
        }
    }

    /// The domain door's primitive. Reserved targets are Outside no
    /// matter what the set says, and a documents domain answers a third
    /// way: inside the prefixes, and still not a file it writes.
    /// Whether a declared area lies inside this domain at all: the
    /// prefix half of [`admits`](Self::admits), for the subject that
    /// has no file name. A tool declares the room it works in, and a
    /// room is neither Markdown nor code.
    #[must_use]
    pub fn reaches(&self, area: &Address) -> bool {
        !area.is_reserved() && self.prefixes().any(|p| area.is_within(p))
    }

    pub fn admits(&self, target: &Address) -> DomainVerdict {
        if !self.reaches(target) {
            return DomainVerdict::Outside {
                prefixes: self.prefix_strings(),
            };
        }
        match self {
            WriteDomain::Everything(_) => DomainVerdict::Within,
            WriteDomain::Documents(_) => document_verdict(target),
        }
    }

    fn prefix_strings(&self) -> Vec<String> {
        self.prefixes().map(|p| p.as_str().to_owned()).collect()
    }
}

/// The last segment of an address: its file name.
fn file_name(target: &Address) -> &str {
    let raw = target.as_str();
    raw.rsplit('/').next().unwrap_or(raw)
}

/// What a documents domain says about one target already known to be
/// inside its prefixes.
fn document_verdict(target: &Address) -> DomainVerdict {
    let name = file_name(target);
    if name == crate::spine::ROADMAP_FILE {
        return DomainVerdict::NotWritable {
            reason: DocumentReason::ThePlan,
        };
    }
    if name.ends_with(".md") {
        DomainVerdict::Within
    } else {
        DomainVerdict::NotWritable {
            reason: DocumentReason::NotMarkdown,
        }
    }
}

/// Deliberately exhaustive verdict; `prefixes` feeds the refusal's
/// nearby list.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainVerdict {
    Within,
    Outside {
        prefixes: Vec<String>,
    },
    /// Inside the prefixes, and still not a file this domain writes.
    NotWritable {
        reason: DocumentReason,
    },
}

/// Why a documents domain refuses a target that is inside its prefixes.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentReason {
    /// Not a Markdown document.
    NotMarkdown,
    /// The plan file, whose one editing entrance is the `plan` tool.
    ThePlan,
}

/// One observed write, in time order (slice order is the clock).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditSample {
    pub addr: Address,
    pub run: RunId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditWarVerdict {
    Calm,
    Freeze { addr: Address },
}

/// Edit-war detection: a "reclaim" is run X editing a
/// file back after run Y took it (X, Y, X). Reclaims at or over
/// `EDIT_WAR_FREEZE` freeze that file. Per-address, deterministic:
/// addresses are examined in BTreeMap order, first hit wins.
pub fn observe_edit_war(samples: &[EditSample]) -> EditWarVerdict {
    let mut per_addr: BTreeMap<&Address, Vec<RunId>> = BTreeMap::new();
    for sample in samples {
        let runs = per_addr.entry(&sample.addr).or_default();
        if runs.last() != Some(&sample.run) {
            runs.push(sample.run);
        }
    }
    for (addr, runs) in per_addr {
        let mut reclaims: u32 = 0;
        for i in 2..runs.len() {
            let (Some(now), Some(prev), Some(before)) = (
                runs.get(i),
                runs.get(i.wrapping_sub(1)),
                runs.get(i.wrapping_sub(2)),
            ) else {
                continue;
            };
            if now == before && now != prev {
                reclaims = reclaims.saturating_add(1);
            }
        }
        if reclaims >= EDIT_WAR_FREEZE {
            return EditWarVerdict::Freeze { addr: addr.clone() };
        }
    }
    EditWarVerdict::Calm
}

#[cfg(kani)]
mod verification {
    //! V5: `admits` is total and reserved targets never pass.

    use super::*;

    #[kani::proof]
    fn reserved_is_never_within() {
        let domain = WriteDomain::new(vec![Address::parse("b").unwrap()]).unwrap();
        let target = Address::parse(".sprawling/ledger").unwrap();
        assert!(matches!(
            domain.admits(&target),
            DomainVerdict::Outside { .. }
        ));
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
mod tests {
    use super::*;

    fn addr(raw: &str) -> Address {
        Address::parse(raw).unwrap()
    }

    fn run(n: u8) -> RunId {
        RunId::parse(&format!("0198f6a2-7c4a-7bbb-9d1e-0000000000{n:02x}")).unwrap()
    }

    #[test]
    fn a_reserved_member_refuses_the_whole_domain() {
        let err = WriteDomain::new(vec![addr("b"), addr(".sprawling/cas")]).unwrap_err();
        assert_eq!(err.code(), &AxCode::InvalidArgs);
    }

    #[test]
    fn admits_within_and_refuses_outside_with_nearby() {
        let domain = WriteDomain::new(vec![addr("b1/room"), addr("b2")]).unwrap();
        assert_eq!(
            domain.admits(&addr("b1/room/file.md")),
            DomainVerdict::Within
        );
        assert_eq!(domain.admits(&addr("b2")), DomainVerdict::Within);
        match domain.admits(&addr("b3/x")) {
            DomainVerdict::Outside { prefixes } => {
                assert_eq!(prefixes, ["b1/room", "b2"]);
            }
            other => panic!("b3/x is outside, not {other:?}"),
        }
    }

    #[test]
    fn reserved_target_is_outside_even_for_an_empty_domain() {
        let domain = WriteDomain::new(vec![]).unwrap();
        assert!(matches!(
            domain.admits(&addr(".sprawling/config")),
            DomainVerdict::Outside { .. }
        ));
        assert!(matches!(
            domain.admits(&addr("anywhere")),
            DomainVerdict::Outside { .. }
        ));
    }

    #[test]
    fn two_full_reclaims_freeze_the_file() {
        let file = addr("b/contested.md");
        let samples: Vec<EditSample> = [run(1), run(2), run(1), run(2)]
            .into_iter()
            .map(|r| EditSample {
                addr: file.clone(),
                run: r,
            })
            .collect();
        assert_eq!(
            observe_edit_war(&samples),
            EditWarVerdict::Freeze { addr: file }
        );
    }

    #[test]
    fn one_reclaim_or_same_run_bursts_stay_calm() {
        let file = addr("b/f.md");
        let one_reclaim: Vec<EditSample> = [run(1), run(2), run(1)]
            .into_iter()
            .map(|r| EditSample {
                addr: file.clone(),
                run: r,
            })
            .collect();
        assert_eq!(observe_edit_war(&one_reclaim), EditWarVerdict::Calm);
        // Consecutive edits by the same run collapse: no alternation, no war.
        let bursts: Vec<EditSample> = [run(1), run(1), run(2), run(2), run(1), run(1)]
            .into_iter()
            .map(|r| EditSample {
                addr: file.clone(),
                run: r,
            })
            .collect();
        assert_eq!(observe_edit_war(&bursts), EditWarVerdict::Calm);
    }

    #[test]
    fn wars_are_per_address_not_global() {
        let samples: Vec<EditSample> = [
            (addr("b/one.md"), run(1)),
            (addr("b/two.md"), run(2)),
            (addr("b/one.md"), run(2)),
            (addr("b/two.md"), run(1)),
            (addr("b/one.md"), run(1)),
            (addr("b/two.md"), run(2)),
        ]
        .into_iter()
        .map(|(a, r)| EditSample { addr: a, run: r })
        .collect();
        // Each file saw only one reclaim; interleaving does not add up.
        assert_eq!(observe_edit_war(&samples), EditWarVerdict::Calm);
    }
}
