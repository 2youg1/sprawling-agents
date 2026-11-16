// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::{Address, AxCode, AxError, ByteLen, GoalId, Payload, PursuitState, TimeMs, Version};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GoalConflict {
    pub goal: GoalId,
    pub with: GoalId,
    pub level: ConflictLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ConflictLevel {
    Serialize,
    Arbitrate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PursuitChanged {
    pub step: PursuitMove,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal: Option<String>,
}

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalEnqueued {
    pub id: SignalId,
    pub kind: SignalKind,
    pub from: String,
    pub room: Address,
    pub room_version: Version,
    pub payload: Payload,
    pub at: TimeMs,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lane: Option<Lane>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalConsumed {
    pub id: SignalId,
    pub by: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalWaitStarted {
    pub on: Address,
    pub signal: SignalId,
    pub deadline_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalWaitEnded {
    pub signal: SignalId,
    pub by: WaitEnd,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "end")]
pub enum WaitEnd {
    Reply { reply: SignalId },

    Timeout,

    Left,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalLanded {
    pub signal: SignalId,
    pub landing: Landing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Landing {
    Delivered,

    Queued,

    Knocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreeOpened {
    pub name: String,
    pub disk_bytes: ByteLen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lane {
    Urgent,
    Ordinary,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SignalId(String);

impl TryFrom<String> for SignalId {
    type Error = AxError;

    fn try_from(raw: String) -> Result<SignalId, AxError> {
        SignalId::parse(&raw)
    }
}

impl From<SignalId> for String {
    fn from(id: SignalId) -> String {
        id.0
    }
}

impl SignalId {
    pub fn parse(raw: &str) -> Result<SignalId, AxError> {
        if raw.is_empty() || raw.chars().any(char::is_whitespace) {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "read a signal id",
                format!("{raw:?}"),
            )
            .with_recovery(
                "use a non-empty id with no whitespace, such as a run id and a counter",
            ));
        }
        Ok(SignalId(raw.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum SignalKind {
    Mention,
    Thread,
    Broadcast,
    Steer,
}

impl SignalKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            SignalKind::Mention => "mention",
            SignalKind::Thread => "thread",
            SignalKind::Broadcast => "broadcast",
            SignalKind::Steer => "steer",
        }
    }

    pub fn parse(raw: &str) -> Result<SignalKind, AxError> {
        match raw {
            "mention" => Ok(SignalKind::Mention),
            "thread" => Ok(SignalKind::Thread),
            "broadcast" => Ok(SignalKind::Broadcast),
            "steer" => Ok(SignalKind::Steer),
            other => {
                Err(
                    AxError::failure(AxCode::InvalidArgs, "read a signal kind", other.to_owned())
                        .with_recovery("mention, thread, broadcast or steer"),
                )
            }
        }
    }
}

impl TryFrom<String> for SignalKind {
    type Error = AxError;

    fn try_from(raw: String) -> Result<SignalKind, AxError> {
        SignalKind::parse(&raw)
    }
}

impl From<SignalKind> for String {
    fn from(kind: SignalKind) -> String {
        kind.as_str().to_owned()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;

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

    #[test]
    fn a_worktree_line_writes_the_keys_the_hand_written_map_wrote() {
        let opened = WorktreeOpened {
            name: "node-1".to_owned(),
            disk_bytes: ByteLen::new(6),
        };
        assert_eq!(
            serde_json::to_string(&Payload::of(&opened).unwrap()).unwrap(),
            r#"{"disk_bytes":6,"name":"node-1"}"#
        );
    }

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
