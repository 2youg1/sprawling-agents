// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The performance monitor's pair of frames (`crates/wire/spec/Frames/Monitor.lean` §8-47g):
//! whether a session is watching, and one reading sent to it; and the
//! sampling beat a page sets for the city (§8-47h).

use kernel::{AxCode, AxError};
use serde::{Deserialize, Serialize};

/// The shortest beat a page may set, in milliseconds.
pub const BEAT_MIN_MS: u32 = 10;
/// The longest beat a page may set, in milliseconds.
pub const BEAT_MAX_MS: u32 = 1_000;

/// How often the monitor reads this process's private bytes, and a tenth
/// of how often it sends a reading (§8-47h). Only a beat inside
/// [`BEAT_MIN_MS`]`..=`[`BEAT_MAX_MS`] can be made, on the wire as off it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BeatMs(u32);

impl BeatMs {
    /// The beat a city samples at until a page sets another (Roadmap M0
    /// item 7).
    pub const DEFAULT: BeatMs = BeatMs(100);

    /// # Errors
    /// `InvalidArgs` for a beat outside the range a page may set.
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

/// Whether this session counts as somebody watching the monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Monitoring {
    /// Watch the whole monitor page.
    Watch,
    /// Watch only the one-line summary beside the settings tree's performance entry, which reads this process alone.
    WatchSummary,
    Release,
    /// Sample at this beat from now on, and remember it for this city.
    Beat(BeatMs),
}

/// What one watcher looks at, which decides how much the city reads for
/// it. It does not travel: it is what `MonitorFeed::watch` is told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Watched {
    Everything,
    Summary,
}

/// One reading of every counter the monitor shows, in integers because
/// it travels on the wire (`crates/sprawling/Spec.lean` §8-94, §8-129-6).
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
    /// Committed records the writer has handed the view thread that are
    /// not yet folded and broadcast (`crates/sprawling/Spec.lean` §8-123).
    pub view_backlog: u64,
    /// How long the beat before this one took to read the counters, by
    /// the sampler's own monotonic clock; 0 on the first beat
    /// (`crates/sprawling/Spec.lean` §8-129-6).
    pub read_nanos: u64,
    /// The beat this reading was sampled at, in milliseconds (§8-47h):
    /// what the page's beat control reads as the current setting.
    pub beat_ms: u64,
}

#[cfg(all(test, feature = "server"))]
#[allow(clippy::unwrap_used, clippy::panic, reason = "test code")]
mod tests {
    use crate::frames::ClientFrame;
    use crate::reception::{BindFace, SessionState, SessionStep, WelcomeFacts, decide_frame};

    /// A beat inside the range reaches the shell as the step that sets it;
    /// one outside it is not a frame at all, so the shell never holds a
    /// beat it has to judge again (§8-47h).
    #[test]
    fn a_beat_in_range_is_a_step_and_one_outside_is_no_frame() {
        let read = |text: &str| serde_json::from_str::<ClientFrame>(text);
        let frame = read(r#"{"monitor":{"beat":250}}"#).unwrap();
        let face = BindFace::Loopback { token: None };
        let step = decide_frame(SessionState::Live, frame, &face, WelcomeFacts::default());
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
