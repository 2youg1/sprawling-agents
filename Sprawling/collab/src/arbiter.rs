// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::{ConflictLevel, GoalConflict};
use kernel::{AxError, GoalEntry, GoalId, GoalResource, GoalVerdict, Payload};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Level {
    Serialize { after: GoalId },

    Arbitrate { with: GoalId },
}

#[must_use]
pub fn arbitrate(registered: &[GoalEntry], candidate: &GoalEntry) -> Option<Level> {
    let GoalVerdict::Conflict { with } = kernel::goal::detect_conflict(registered, candidate)
    else {
        return None;
    };
    let held = registered.iter().find(|entry| entry.id == with);

    let mechanical = held.is_some_and(|entry| {
        entry.standing != candidate.standing
            && entry
                .resources
                .iter()
                .chain(candidate.resources.iter())
                .all(|resource| matches!(resource, GoalResource::Path(_)))
    });
    if mechanical {
        return Some(Level::Serialize { after: with });
    }
    Some(Level::Arbitrate { with })
}

pub fn conflict_payload(candidate: &GoalEntry, level: &Level) -> Result<Payload, AxError> {
    let (with, level) = match level {
        Level::Serialize { after } => (after, ConflictLevel::Serialize),
        Level::Arbitrate { with } => (with, ConflictLevel::Arbitrate),
    };
    Payload::of(&GoalConflict {
        goal: candidate.id.clone(),
        with: with.clone(),
        level,
    })
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
    use kernel::Address;

    fn goal(id: &str, path: &str, standing: bool) -> GoalEntry {
        GoalEntry {
            id: GoalId::new(id).unwrap(),
            owner: format!("lab/{id}"),
            resources: vec![GoalResource::Path(Address::parse(path).unwrap())],
            statement: format!("finish {id}"),
            standing,
        }
    }

    fn external(id: &str, name: &str) -> GoalEntry {
        GoalEntry {
            id: GoalId::new(id).unwrap(),
            owner: format!("lab/{id}"),
            resources: vec![GoalResource::External(name.to_owned())],
            statement: format!("use {name}"),
            standing: false,
        }
    }

    #[test]
    fn goals_that_do_not_touch_have_nothing_to_settle() {
        let registered = vec![goal("a", "lab/one", true)];
        assert_eq!(arbitrate(&registered, &goal("b", "lab/two", false)), None);
    }

    #[test]
    fn a_clash_a_machine_can_settle_is_settled_by_waiting() {
        let registered = vec![goal("a", "lab/one", true)];
        assert_eq!(
            arbitrate(&registered, &goal("b", "lab/one/deeper", false)),
            Some(Level::Serialize {
                after: GoalId::new("a").unwrap()
            })
        );
    }

    #[test]
    fn a_clash_that_needs_reading_goes_to_a_resident() {
        let registered = vec![goal("a", "lab/one", true)];
        assert_eq!(
            arbitrate(&registered, &goal("b", "lab/one", true)),
            Some(Level::Arbitrate {
                with: GoalId::new("a").unwrap()
            })
        );

        let registered = vec![external("a", "the printer")];
        assert!(matches!(
            arbitrate(&registered, &external("b", "the printer")),
            Some(Level::Arbitrate { .. })
        ));
    }

    #[test]
    fn the_record_says_who_clashed_and_how_far_up_it_went() {
        let level = Level::Arbitrate {
            with: GoalId::new("a").unwrap(),
        };
        let read = |level: &Level| {
            conflict_payload(&goal("b", "lab/one", true), level)
                .unwrap()
                .read::<GoalConflict>()
                .unwrap()
        };
        let a = GoalId::new("a").unwrap();
        assert_eq!(
            (read(&level), read(&Level::Serialize { after: a.clone() })),
            (
                GoalConflict {
                    goal: GoalId::new("b").unwrap(),
                    with: a.clone(),
                    level: ConflictLevel::Arbitrate,
                },
                GoalConflict {
                    goal: GoalId::new("b").unwrap(),
                    with: a,
                    level: ConflictLevel::Serialize,
                },
            )
        );
    }
}
