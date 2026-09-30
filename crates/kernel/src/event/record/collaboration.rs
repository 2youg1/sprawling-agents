// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What residents did together: the goals a building pursues, the
//! ground two goals met on, and the signals they send one another.
//!
//! `goal_registered` has no struct here: its payload is the
//! [`GoalEntry`](crate::GoalEntry) the goal register holds, written and
//! read through `Payload::of` and `Payload::read` like every record.

use serde::{Deserialize, Serialize};

use crate::{Address, AxCode, AxError, ByteLen, GoalId, Payload, PursuitState, TimeMs, Version};

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

/// `signal_enqueued`: the signal itself, and the line it waits in.
///
/// `collab::Signal` writes it and reads it back; the queue rebuilt after
/// a restart and every view of what is waiting read the same struct, so
/// the seven keys have one spelling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalEnqueued {
    pub id: SignalId,
    pub kind: SignalKind,
    pub from: String,
    pub room: Address,
    pub room_version: Version,
    pub payload: Payload,
    pub at: TimeMs,
    /// Written for a reader outside collab and never trusted on the way
    /// back: the lane is derived from `kind`, and reading a stored copy
    /// would let a line say which lane it took while the derivation says
    /// another. Absent on a line written before the key existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lane: Option<Lane>,
}

/// `signal_consumed`: which signal was taken, and by whom. The content
/// is already in the enqueue line, and history does not need it twice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalConsumed {
    pub id: SignalId,
    pub by: String,
}

/// `worktree_opened`: the tree a resident was given to work a claimed
/// node in, by name and measured size. It carries no path: an absolute
/// path is a fact about one machine, and a history that holds one does
/// not survive being moved to another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreeOpened {
    pub name: String,
    pub disk_bytes: ByteLen,
}

/// Which line a signal waits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lane {
    Urgent,
    Ordinary,
}

/// A signal's identity, and the thing duplicates are recognised by.
///
/// It serializes as the string [`SignalId::parse`] accepted and reads
/// back through that same parse, so a ledger line carrying an id meets
/// the grammar once rather than once per reader.
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
    /// # Errors
    /// Refuses an empty id and one carrying whitespace: an id is
    /// compared, logged and replayed, and all three go wrong quietly
    /// when it can contain a space.
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

/// What kind of communication a signal is. `Steer` is a fourth kind
/// rather than a flag beside the other three: it is the only one that
/// overtakes, and urgency has to belong to the signal for one id to
/// always take one lane.
///
/// Its serde form goes through [`SignalKind::as_str`] and
/// [`SignalKind::parse`] rather than through a derived renaming, so the
/// four wire words are spelled in one place.
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

    /// # Errors
    /// Refuses a kind this version does not know.
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

    /// The bytes `accounting::worker::plans` wrote by hand: `step` always,
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

    /// The bytes `storage`'s worktree lease wrote by hand: a name and a
    /// size, and no path.
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
