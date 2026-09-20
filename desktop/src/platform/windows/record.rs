// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Start and stop: which windows this connection is recording, and
//! what each recording is being written by.
//!
//! This file is the bookkeeping. What writes the bytes and where they
//! land is `sink`'s, so the two questions a reader asks separately are
//! answered separately: *may this recording begin* here, *how is it
//! written* there.
//!
//! A recording cannot outlive the connection that started it: the desk
//! is dropped with the connection, and dropping it stops what it was
//! running.
//!
//! Two recordings of one window at once is refused rather than merged.
//! Whichever of the two files a caller then asked for would be a guess,
//! and stopping would be ambiguous in a way no answer resolves.
//!
//! Sound is refused. Choosing a capture device requires knowing what it
//! is called on this machine, and nothing in this server knows that; a
//! recording that silently had no audio track would be discovered by
//! whoever played it back (desktop-SPEC.md §8.6, fourth pair).

mod sink;

use std::collections::BTreeMap;
use std::path::PathBuf;

use windows::Win32::Foundation::HWND;

use super::geometry::Bounds;
use crate::refusal::{Refusal, RefusalCode};
use sink::{Sink, somewhere};

/// One recording in progress.
struct Running {
    into: PathBuf,
    written_by: Sink,
}

/// Every recording this connection started, by the window it is of.
#[derive(Default)]
pub(crate) struct Recordings {
    running: BTreeMap<String, Running>,
    /// How many recordings this desk has begun, which is what makes two
    /// recordings of one window land in two directories. A counter
    /// rather than a timestamp because this package samples no clock,
    /// and because a counter cannot collide with itself.
    begun: u64,
}

impl Recordings {
    pub(crate) fn new() -> Recordings {
        Recordings {
            running: BTreeMap::new(),
            begun: 0,
        }
    }

    /// Begins recording one window.
    ///
    /// # Errors
    /// Refuses a second recording of the same window, a request for
    /// sound, and a place on this machine that cannot be written to.
    pub(crate) fn start(
        &mut self,
        window: &str,
        handle: HWND,
        bounds: Bounds,
        audio: bool,
    ) -> Result<PathBuf, Refusal> {
        if audio {
            return Err(Refusal::new(
                RefusalCode::ToolUnavailable,
                "record a window",
                "this server does not choose a sound device".to_owned(),
                "record without `audio`; a recording that claimed to have sound and had none \
                 would be discovered by whoever played it back",
            ));
        }
        if self.running.contains_key(window) {
            return Err(Refusal::new(
                RefusalCode::GateDenied,
                "record a window",
                format!("`{window}` is already being recorded"),
                "stop the recording that is running before starting another; two of one window \
                 would leave `stop` with two files and no way to say which one it meant",
            ));
        }
        self.begun = self.begun.saturating_add(1);
        let into = somewhere(window, self.begun)?;
        let written_by = Sink::open(window, handle, bounds, &into);
        self.running
            .insert(window.to_owned(), Running { into, written_by });
        self.running
            .get(window)
            .map(|running| running.into.clone())
            .ok_or_else(|| {
                Refusal::new(
                    RefusalCode::ToolUnavailable,
                    "record a window",
                    "the recording could not be recorded as started".to_owned(),
                    "this is a defect in this server rather than in the call; report it",
                )
            })
    }

    /// Ends the recording of one window and answers with where it
    /// landed.
    ///
    /// # Errors
    /// Refuses a stop for a window nothing is recording, which is the
    /// honest answer to "stop what?".
    pub(crate) fn stop(&mut self, window: &str) -> Result<(PathBuf, &'static str), Refusal> {
        let Some(running) = self.running.remove(window) else {
            return Err(Refusal::new(
                RefusalCode::InvalidArgs,
                "record a window",
                format!("nothing is being recorded of `{window}`"),
                "send `state: start` first; this server does not stop on its own, so a recording \
                 that was never started has nothing to stop",
            ));
        };
        Ok((running.into, running.written_by.close()))
    }
}

impl Drop for Recordings {
    /// A connection that closes mid-recording stops what it started.
    /// Leaving a thread grabbing frames of somebody's desktop after the
    /// caller has gone is the kind of thing this server exists to not do.
    fn drop(&mut self) {
        let windows: Vec<String> = self.running.keys().cloned().collect();
        for window in windows {
            let _stopped = self.stop(&window);
        }
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

    fn bounds() -> Bounds {
        Bounds::from_corners(0, 0, 32, 32).unwrap()
    }

    fn nowhere() -> HWND {
        HWND(std::ptr::null_mut())
    }

    /// The two bookkeeping rules, which hold whatever this machine has
    /// installed: one recording per window, and nothing to stop until
    /// something started.
    #[test]
    fn one_window_records_once_and_stops_once() {
        let mut recordings = Recordings::new();
        let unstarted = recordings
            .stop("Calculator")
            .expect_err("nothing is recording");
        assert_eq!(unstarted.as_error()["data"]["code"], "E_INVALID_ARGS");
        assert!(
            unstarted.as_error()["data"]["recovery"]
                .as_str()
                .unwrap()
                .contains("state: start")
        );

        let into = recordings
            .start("Calculator", nowhere(), bounds(), false)
            .expect("a recording starts even of a window that draws nothing");
        assert!(into.is_dir(), "{} was not laid out", into.display());

        let twice = recordings
            .start("Calculator", nowhere(), bounds(), false)
            .expect_err("two recordings of one window is not a thing `stop` can answer");
        assert_eq!(twice.as_error()["data"]["code"], "E_GATE_DENIED");

        let (landed, how) = recordings.stop("Calculator").expect("the recording ends");
        assert_eq!(landed, into);
        assert!(matches!(how, "mp4" | "frames"), "{how}");
        // Stopping twice is the same refusal as stopping something that
        // never started, rather than a second file.
        assert!(recordings.stop("Calculator").is_err());
        let _tidied = std::fs::remove_dir_all(&into);
    }

    /// Sound is refused with the reason, not accepted and dropped.
    #[test]
    fn asking_for_sound_is_refused_rather_than_silently_ignored() {
        let mut recordings = Recordings::new();
        let refusal = recordings
            .start("Calculator", nowhere(), bounds(), true)
            .expect_err("this server chooses no sound device");
        let error = refusal.as_error();
        assert_eq!(error["data"]["code"], "E_TOOL_UNAVAILABLE");
        assert!(
            error["data"]["recovery"]
                .as_str()
                .unwrap()
                .contains("without `audio`")
        );
        assert!(recordings.running.is_empty(), "a refusal started nothing");
    }

    /// Two windows record independently, and neither stop reaches the
    /// other.
    #[test]
    fn two_windows_record_and_stop_independently() {
        let mut recordings = Recordings::new();
        let one = recordings
            .start("Calculator", nowhere(), bounds(), false)
            .unwrap();
        let two = recordings
            .start("a.txt — Notepad", nowhere(), bounds(), false)
            .unwrap();
        assert_ne!(one, two);
        assert!(recordings.stop("Calculator").is_ok());
        assert!(recordings.stop("Calculator").is_err());
        assert!(recordings.stop("a.txt — Notepad").is_ok());
        for landed in [one, two] {
            let _tidied = std::fs::remove_dir_all(&landed);
        }
    }

    /// Dropping the desk stops what it was running, so a connection that
    /// closes does not leave a thread grabbing frames of somebody's
    /// desktop. If the drop did not join the thread, this is the test
    /// that would hang rather than the one that would fail.
    #[test]
    fn dropping_the_table_stops_every_recording_it_held() {
        let landed = {
            let mut recordings = Recordings::new();
            let landed = recordings
                .start("a window only this test names", nowhere(), bounds(), false)
                .unwrap();
            assert_eq!(recordings.running.len(), 1);
            landed
        };
        assert!(landed.is_dir(), "{} was not laid out", landed.display());
        let _tidied = std::fs::remove_dir_all(&landed);
    }
}
