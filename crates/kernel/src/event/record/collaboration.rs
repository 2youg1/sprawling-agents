// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What residents did together: the goals a building pursues, and the
//! ground two goals met on.
//!
//! `goal_registered` has no struct here: its payload is the
//! [`GoalEntry`](crate::GoalEntry) the goal register holds, written and
//! read through `Payload::of` and `Payload::read` like every record.

use serde::{Deserialize, Serialize};

use crate::{AxCode, AxError, GoalId, PursuitState};

/// `goal_conflict`: a goal that asked for ground another goal holds,
/// and how far up the settling had to go.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GoalConflict {
    pub goal: GoalId,
    pub with: GoalId,
    pub level: ConflictLevel,
}

/// Which level settles a `goal_conflict`: `serialize` runs the goal
/// after the one it met, `arbitrate` hands both statements to a
/// resident.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ConflictLevel {
    Serialize,
    Arbitrate,
}

/// `pursuit_changed`: one step a person took on a building's pursuit,
/// and the goal the building holds after it.
///
/// `goal` is absent exactly when the step left no pursuit, which is a
/// `clear`; every other step leaves one and writes its goal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PursuitChanged {
    pub step: PursuitMove,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal: Option<String>,
}

/// Which step a `pursuit_changed` line records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PursuitMove {
    Set,
    Pause,
    Resume,
    Clear,
}

impl PursuitChanged {
    /// The goal and state the building holds after this step, or `None`
    /// when the step cleared it.
    ///
    /// # Errors
    /// Refuses a step that leaves a pursuit but names no goal: a reader
    /// that invented an empty goal for it would hold a pursuit nobody
    /// declared.
    pub fn held(self) -> Result<Option<(String, PursuitState)>, AxError> {
        let state = match self.step {
            PursuitMove::Set | PursuitMove::Resume => PursuitState::Running,
            PursuitMove::Pause => PursuitState::Paused,
            PursuitMove::Clear => return Ok(None),
        };
        let goal = self.goal.ok_or_else(|| {
            AxError::failure(
                AxCode::WireMismatch,
                "read a pursuit_changed line",
                "a step that leaves a pursuit names no goal",
            )
            .with_recovery("replay with the build that wrote this record")
        })?;
        Ok(Some((goal, state)))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;
    use crate::event::Payload;

    /// The bytes `bin::assembly::plans` wrote by hand: `step` always,
    /// `goal` only while a pursuit is still held.
    #[test]
    fn a_pursuit_step_writes_the_keys_the_hand_written_map_wrote() {
        let hand = |json: serde_json::Value| {
            serde_json::to_string(&Payload::new(json.as_object().unwrap().clone()).unwrap())
                .unwrap()
        };
        let typed =
            |record: &PursuitChanged| serde_json::to_string(&Payload::of(record).unwrap()).unwrap();
        let paused = PursuitChanged {
            step: PursuitMove::Pause,
            goal: Some("read the meter".to_owned()),
        };
        let cleared = PursuitChanged {
            step: PursuitMove::Clear,
            goal: None,
        };
        assert_eq!(
            (typed(&paused), typed(&cleared)),
            (
                hand(serde_json::json!({ "step": "pause", "goal": "read the meter" })),
                hand(serde_json::json!({ "step": "clear" })),
            )
        );
        assert_eq!(
            (paused.held().unwrap(), cleared.held().unwrap()),
            (
                Some(("read the meter".to_owned(), PursuitState::Paused)),
                None
            )
        );
    }

    /// The bytes `collab::arbiter::conflict_payload` wrote by hand, and
    /// the `goal_registered` entry, which serde wrote from `GoalEntry`.
    #[test]
    fn a_goal_line_writes_the_keys_the_hand_written_map_wrote() {
        let conflict = GoalConflict {
            goal: GoalId::new("b").unwrap(),
            with: GoalId::new("a").unwrap(),
            level: ConflictLevel::Arbitrate,
        };
        let entry = crate::GoalEntry {
            id: GoalId::new("a").unwrap(),
            owner: "lab/a".to_owned(),
            resources: vec![crate::GoalResource::External("the printer".to_owned())],
            statement: "print".to_owned(),
            standing: true,
        };
        let bytes = |payload: Payload| serde_json::to_string(&payload).unwrap();
        assert_eq!(
            (
                bytes(Payload::of(&conflict).unwrap()),
                bytes(Payload::of(&entry).unwrap()),
            ),
            (
                r#"{"goal":"b","level":"arbitrate","with":"a"}"#.to_owned(),
                r#"{"id":"a","owner":"lab/a","resources":[{"external":"the printer"}],"standing":true,"statement":"print"}"#.to_owned(),
            )
        );
        assert_eq!(
            Payload::of(&conflict)
                .unwrap()
                .read::<GoalConflict>()
                .unwrap(),
            conflict
        );
    }
}
