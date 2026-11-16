// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError};
use serde::{Deserialize, Serialize};

pub const BEAT_MIN_MS: u32 = 10;

pub const BEAT_MAX_MS: u32 = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BeatMs(u32);

impl BeatMs {
    pub const DEFAULT: BeatMs = BeatMs(100);

    pub fn new(ms: u32) -> Result<BeatMs, AxError> {
        if (BEAT_MIN_MS..=BEAT_MAX_MS).contains(&ms) {
            Ok(BeatMs(ms))
        } else {
            Err(AxError::failure(
                AxCode::InvalidArgs,
                "set the monitor's beat",
                format!("{ms} ms"),
            )
            .with_recovery(format!(
                "choose a beat from {BEAT_MIN_MS} to {BEAT_MAX_MS} milliseconds"
            )))
        }
    }

    #[must_use]
    pub fn ms(self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for BeatMs {
    type Error = AxError;

    fn try_from(ms: u32) -> Result<BeatMs, AxError> {
        BeatMs::new(ms)
    }
}

impl From<BeatMs> for u32 {
    fn from(beat: BeatMs) -> u32 {
        beat.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Monitoring {
    Watch,

    WatchSummary,
    Release,

    Beat(BeatMs),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Watched {
    Everything,
    Summary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Sample {
    pub core_cpu_permille: u64,
    pub core_private_bytes: u64,
    pub core_working_set_bytes: u64,
    pub core_read_bytes: u64,
    pub core_written_bytes: u64,
    pub machine_cpu_permille: u64,
    pub machine_available_bytes: u64,
    pub volume_free_bytes: u64,
    pub ledger_queue_depth: u64,
    pub durable_lag: u64,
    pub relay_p50_nanos: u64,
    pub event_to_screen_p50_nanos: u64,
    pub queued_runs: u64,

    pub view_backlog: u64,

    pub read_nanos: u64,

    pub beat_ms: u64,
}

#[cfg(all(test, feature = "server"))]
#[allow(clippy::unwrap_used, clippy::panic, reason = "test code")]
mod tests {
    use crate::frames::ClientFrame;
    use crate::reception::{BindFace, Keys, SessionState, SessionStep, WelcomeFacts, decide_frame};

    #[test]
    fn a_beat_in_range_is_a_step_and_one_outside_is_no_frame() {
        let read = |text: &str| serde_json::from_str::<ClientFrame>(text);
        let frame = read(r#"{"monitor":{"beat":250}}"#).unwrap();
        let face = BindFace::Loopback {
            key: kernel::B3Hash::digest(b"native-key"),
        };
        let sessions = crate::reception::Sessions::default();
        let keys = Keys {
            face: &face,
            sessions: &sessions,
            now: kernel::TimeMs::new(0),
        };
        let step = decide_frame(SessionState::Live, frame, &keys, WelcomeFacts::default());
        let SessionStep::Beat(beat) = step else {
            panic!("a live session's beat is a step: {step:?}");
        };
        assert_eq!(
            (
                beat.ms(),
                read(r#"{"monitor":{"beat":5}}"#).is_err(),
                read(r#"{"monitor":{"beat":1001}}"#).is_err()
            ),
            (250, true, true)
        );
    }
}
