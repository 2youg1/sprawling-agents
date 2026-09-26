// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What residents did together: the goals a building pursues.

use serde::{Deserialize, Serialize};

use crate::{AxCode, AxError, PursuitState};

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
}
