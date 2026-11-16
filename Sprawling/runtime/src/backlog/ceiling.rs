// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::num::NonZeroU64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ceiling {

    Hit {

        limit: NonZeroU64,
    },

    Unapplied {

        limit: NonZeroU64,

        why: Unapplied,
    },

    Unread {

        limit: NonZeroU64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unapplied {

    Platform,

    NotDelegated,

    Refused,

    Unjoined,
}

impl Unapplied {

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Mark {

    NotAsked,

    Unapplied { limit: NonZeroU64, why: Unapplied },

    Unread { limit: NonZeroU64 },

    Watching { limit: NonZeroU64, before: u64 },
}

impl Mark {

    pub(super) fn entered(limit: NonZeroU64, count: Option<u64>) -> Mark {
        match count {
            Some(before) => Mark::Watching { limit, before },
            None => Mark::Unread { limit },
        }
    }

    pub(super) fn outside(limit: Option<NonZeroU64>, why: Unapplied) -> Mark {
        match limit {
            Some(limit) => Mark::Unapplied { limit, why },
            None => Mark::NotAsked,
        }
    }

    pub(super) fn watching(self) -> bool {
        matches!(self, Mark::Watching { .. })
    }
}

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

        #[test]
        fn a_hit_is_reported_only_when_the_count_moved(mark in marks(), count in any::<Option<u64>>()) {
            let hit = matches!(verdict(mark, count), Some(Ceiling::Hit { .. }));
            let moved = matches!((mark, count), (Mark::Watching { before, .. }, Some(after)) if before < after);
            prop_assert_eq!(hit, moved);
        }

        #[test]
        fn an_asked_ceiling_is_silent_only_when_held_and_unhit(mark in marks(), count in any::<Option<u64>>()) {
            let silent = verdict(mark, count).is_none();
            let held = matches!((mark, count), (Mark::Watching { before, .. }, Some(after)) if after <= before);
            prop_assert_eq!(silent, mark == Mark::NotAsked || held);
        }
    }
}
