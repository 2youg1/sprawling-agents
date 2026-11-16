// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::delegation::Delegator;
use crate::node_id::NodeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PursuitState {
    Running,
    Paused,
}

#[derive(Debug)]
pub struct Pursuit {
    goal: String,
    state: PursuitState,
}

impl Pursuit {
    pub fn declare(_at: &Delegator, goal: String) -> Result<Pursuit, crate::error::AxError> {
        if goal.trim().is_empty() {
            return Err(crate::error::AxError::failure(
                crate::error::AxCode::InvalidArgs,
                "declare a pursuit",
                "the goal is blank",
            )
            .with_recovery("say what the city is working towards, in one sentence"));
        }
        Ok(Pursuit {
            goal,
            state: PursuitState::Running,
        })
    }

    #[must_use]
    pub fn goal(&self) -> &str {
        &self.goal
    }

    #[must_use]
    pub fn state(&self) -> PursuitState {
        self.state
    }

    pub fn pause(&mut self) {
        self.state = PursuitState::Paused;
    }

    pub fn resume(&mut self) {
        self.state = PursuitState::Running;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PursuitVerdict {
    Work { next: NodeId },

    Waiting { in_flight: u32 },

    Paused,

    Finished,
}

#[must_use]
pub fn observe(state: PursuitState, ready: &[NodeId], in_flight: u32) -> PursuitVerdict {
    if state == PursuitState::Paused {
        return PursuitVerdict::Paused;
    }
    match ready.first() {
        Some(next) => PursuitVerdict::Work { next: next.clone() },
        None if in_flight > 0 => PursuitVerdict::Waiting { in_flight },
        None => PursuitVerdict::Finished,
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

    fn goal() -> Pursuit {
        Pursuit::declare(&Delegator::root(), "raise the east wing".to_owned()).unwrap()
    }

    fn node(raw: &str) -> NodeId {
        NodeId::parse(raw).unwrap()
    }

    #[test]
    fn work_is_taken_from_the_front_of_the_ready_set() {
        let verdict = observe(goal().state(), &[node("1.2"), node("3")], 0);
        assert_eq!(verdict, PursuitVerdict::Work { next: node("1.2") });
    }

    #[test]
    fn a_city_stops_when_nothing_is_ready_and_nobody_is_working() {
        assert_eq!(
            observe(goal().state(), &[], 2),
            PursuitVerdict::Waiting { in_flight: 2 }
        );
        assert_eq!(observe(goal().state(), &[], 0), PursuitVerdict::Finished);
    }

    #[test]
    fn a_paused_goal_takes_nothing_and_keeps_what_it_was_for() {
        let mut pursuit = goal();
        pursuit.pause();
        assert_eq!(
            observe(pursuit.state(), &[node("1")], 0),
            PursuitVerdict::Paused
        );
        assert_eq!(pursuit.goal(), "raise the east wing");
        pursuit.resume();
        assert_eq!(
            observe(pursuit.state(), &[node("1")], 0),
            PursuitVerdict::Work { next: node("1") }
        );
    }

    #[test]
    fn a_blank_goal_is_refused_where_it_is_declared() {
        let refusal = Pursuit::declare(&Delegator::root(), "   ".to_owned()).unwrap_err();
        assert_eq!(refusal.code(), &crate::error::AxCode::InvalidArgs);
        assert!(refusal.recovery().contains("one sentence"));
    }
}
