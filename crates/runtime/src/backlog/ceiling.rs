// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a command's result says about the person's memory ceiling
//! (`crates/runtime/spec/Tools/Exec.lean` D95): the mark a command gets
//! as it enters the table, and the verdict read when it is collected.
//! The platforms only count refusals ([`super::jobs`] on Windows,
//! [`super::cgroup`] on Linux); which case is reported is decided here
//! and nowhere else.

use std::num::NonZeroU64;

/// How the person's memory ceiling fared while one command ran.
///
/// No variant means "held and not reached": a command whose ceiling held
/// carries no entry at all, so the model reads nothing it has to discard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ceiling {
    /// While the command ran, an allocation of the run's processes was
    /// refused at the ceiling.
    Hit {
        /// The bytes the run's processes may commit together.
        limit: NonZeroU64,
    },
    /// The ceiling was asked for, and the command ran outside it.
    Unapplied {
        /// The bytes the person entered.
        limit: NonZeroU64,
        /// Why the command ran outside it.
        why: Unapplied,
    },
    /// The ceiling was set, and whether it was reached cannot be read.
    Unread {
        /// The bytes the run's processes may commit together.
        limit: NonZeroU64,
    },
}

/// Why a command ran outside the ceiling the person asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unapplied {
    /// This platform has no per-run memory ceiling (macOS).
    Platform,
    /// The Linux cgroup this process sits in is not delegated to it.
    NotDelegated,
    /// The job or the cgroup refused the ceiling or the means to read it.
    Refused,
    /// The command did not join its run's job or cgroup.
    Unjoined,
}

impl Unapplied {
    /// The spelling a result carries.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Unapplied::Platform => "platform",
            Unapplied::NotDelegated => "not_delegated",
            Unapplied::Refused => "refused",
            Unapplied::Unjoined => "unjoined",
        }
    }
}

/// What a command was given as it entered the table: the Rust face of
/// `CeilingMark` in the Lean part.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Mark {
    /// No ceiling was asked for.
    NotAsked,
    /// A ceiling was asked for, and this command runs outside it.
    Unapplied { limit: NonZeroU64, why: Unapplied },
    /// The ceiling holds, and its refusal count did not read at entry.
    Unread { limit: NonZeroU64 },
    /// The ceiling holds, and its run had counted `before` refusals.
    Watching { limit: NonZeroU64, before: u64 },
}

impl Mark {
    /// The mark of a command that entered a ceiling whose refusal count
    /// read `count` at that moment.
    pub(super) fn entered(limit: NonZeroU64, count: Option<u64>) -> Mark {
        match count {
            Some(before) => Mark::Watching { limit, before },
            None => Mark::Unread { limit },
        }
    }

    /// The mark of a command that asked for `limit`, or nothing, and ran
    /// outside it for `why`.
    pub(super) fn outside(limit: Option<NonZeroU64>, why: Unapplied) -> Mark {
        match limit {
            Some(limit) => Mark::Unapplied { limit, why },
            None => Mark::NotAsked,
        }
    }

    /// Whether the verdict needs the run's refusal count read again.
    pub(super) fn watching(self) -> bool {
        matches!(self, Mark::Watching { .. })
    }
}

/// The report for a command collected when its run's refusal count
/// reads `count` (`ceilingAt` in the Lean part): a hit only when the
/// count moved past the one at entry, and every asked ceiling that is not
/// held and unhit says so.
pub(super) fn verdict(mark: Mark, count: Option<u64>) -> Option<Ceiling> {
    match mark {
        Mark::NotAsked => None,
        Mark::Unapplied { limit, why } => Some(Ceiling::Unapplied { limit, why }),
        Mark::Unread { limit } => Some(Ceiling::Unread { limit }),
        Mark::Watching { limit, before } => match count {
            None => Some(Ceiling::Unread { limit }),
            Some(after) if before < after => Some(Ceiling::Hit { limit }),
            Some(_) => None,
        },
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn marks() -> impl Strategy<Value = Mark> {
        let limit = (1_u64..=u64::MAX).prop_map(|bytes| NonZeroU64::new(bytes).unwrap());
        let why = prop_oneof![
            Just(Unapplied::Platform),
            Just(Unapplied::NotDelegated),
            Just(Unapplied::Refused),
            Just(Unapplied::Unjoined),
        ];
        prop_oneof![
            Just(Mark::NotAsked),
            (limit.clone(), why).prop_map(|(limit, why)| Mark::Unapplied { limit, why }),
            limit.clone().prop_map(|limit| Mark::Unread { limit }),
            (limit, any::<u64>()).prop_map(|(limit, before)| Mark::Watching { limit, before }),
        ]
    }

    proptest! {
        /// `hit_only_when_the_count_moved`: a hit is reported exactly
        /// when the ceiling was watched and the count read past its entry.
        #[test]
        fn a_hit_is_reported_only_when_the_count_moved(mark in marks(), count in any::<Option<u64>>()) {
            let hit = matches!(verdict(mark, count), Some(Ceiling::Hit { .. }));
            let moved = matches!((mark, count), (Mark::Watching { before, .. }, Some(after)) if before < after);
            prop_assert_eq!(hit, moved);
        }

        /// `an_asked_ceiling_is_silent_only_when_held_and_unhit`: an asked
        /// ceiling says nothing only when it was watched, read, and unmoved.
        #[test]
        fn an_asked_ceiling_is_silent_only_when_held_and_unhit(mark in marks(), count in any::<Option<u64>>()) {
            let silent = verdict(mark, count).is_none();
            let held = matches!((mark, count), (Mark::Watching { before, .. }, Some(after)) if after <= before);
            prop_assert_eq!(silent, mark == Mark::NotAsked || held);
        }
    }
}
