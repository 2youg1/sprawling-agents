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
//! **A recording is named by the id `start` handed back, not by the
//! window's title.** A title changes while the recording runs — a
//! document is saved under a new name, a tab is switched — and a key
//! that changes under the recording is a recording the caller can no
//! longer stop. The window itself is identified by its handle, which is
//! what makes "this window is already being recorded" hold across a
//! title change too.
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

use serde_json::{Value, json};

use super::enumerate::Window;
use crate::refusal::{Refusal, RefusalCode};
use sink::{Sink, somewhere};

/// One recording of this connection, as the caller names it.
///
/// A counter rather than a timestamp because this package samples no
/// clock, and a counter cannot collide with itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RecordingId(u64);

/// What a `desktop.record` call asks for.
pub(crate) enum Wanted {
    /// Begin recording the window this call names.
    Start,
    /// End the recording this id was handed back for.
    Stop(RecordingId),
}

/// Reads one `desktop.record` call.
///
/// # Errors
/// Refuses a call that says neither start nor stop, a request for
/// sound, and a stop that does not say which recording it ends.
pub(crate) fn asked(arguments: &Value) -> Result<Wanted, Refusal> {
    let state = arguments
        .get("state")
        .and_then(Value::as_str)
        .ok_or_else(|| named_neither("the call says neither start nor stop".to_owned()))?;
    match state {
        "start" => {
            // Absent and `false` are the only two ways to ask for no
            // sound. Anything else is a caller who believes it asked
            // for sound, and is told that it did not get it.
            if !matches!(arguments.get("audio"), None | Some(Value::Bool(false))) {
                return Err(Refusal::new(
                    RefusalCode::ToolUnavailable,
                    "record a window",
                    "this server does not choose a sound device".to_owned(),
                    "record without `audio`; a recording that claimed to have sound and had \
                     none would be discovered by whoever played it back",
                ));
            }
            Ok(Wanted::Start)
        }
        "stop" => match arguments.get("recording").and_then(Value::as_u64) {
            Some(id) => Ok(Wanted::Stop(RecordingId(id))),
            None => Err(Refusal::new(
                RefusalCode::InvalidArgs,
                "record a window",
                "the call does not say which recording to stop".to_owned(),
                "send `recording` with the id `start` answered with; a window's title can \
                 change while it is being recorded, so the title does not name the recording",
            )),
        },
        other => Err(named_neither(format!(
            "`{other}` is neither start nor stop"
        ))),
    }
}

fn named_neither(subject: String) -> Refusal {
    Refusal::new(
        RefusalCode::InvalidArgs,
        "record a window",
        subject,
        "send `state: start` or `state: stop`",
    )
}

/// One recording in progress.
struct Running {
    /// The window being recorded, by the only name of it that cannot
    /// change: the handle the operating system gave it.
    of: usize,
    title: String,
    into: PathBuf,
    written_by: Sink,
}

/// Every recording this connection started, by the id it was given.
#[derive(Default)]
pub(crate) struct Recordings {
    running: BTreeMap<RecordingId, Running>,
    /// How many recordings this desk has begun, which is both the next
    /// id and what makes two recordings of one window land in two
    /// directories.
    begun: u64,
}

impl Recordings {
    pub(crate) fn new() -> Recordings {
        Recordings {
            running: BTreeMap::new(),
            begun: 0,
        }
    }

    /// Begins recording one window and answers with the id that ends
    /// it.
    ///
    /// # Errors
    /// Refuses a second recording of the same window and a place on
    /// this machine that cannot be written to.
    pub(crate) fn start(&mut self, window: &Window) -> Result<Value, Refusal> {
        let of = window.handle.0.addr();
        if let Some(already) = self.running.values().find(|running| running.of == of) {
            return Err(Refusal::new(
                RefusalCode::GateDenied,
                "record a window",
                format!("`{}` is already being recorded", already.title),
                "stop the recording that is running before starting another; two of one window \
                 would leave `stop` with two files and no way to say which one it meant",
            ));
        }
        self.begun = self.begun.saturating_add(1);
        let id = RecordingId(self.begun);
        let into = somewhere(&window.named.title, self.begun)?;
        let written_by = Sink::open(&window.named.title, window.handle, window.bounds, &into);
        let answer = json!({
            "state": "started",
            "recording": id.0,
            "title": window.named.title,
            "into": into.display().to_string(),
        });
        self.running.insert(
            id,
            Running {
                of,
                title: window.named.title.clone(),
                into,
                written_by,
            },
        );
        Ok(answer)
    }

    /// Ends one recording and answers with where it landed.
    ///
    /// # Errors
    /// Refuses an id this connection is not recording under, which is
    /// the honest answer to "stop what?".
    pub(crate) fn stop(&mut self, id: RecordingId) -> Result<Value, Refusal> {
        let Some(running) = self.running.remove(&id) else {
            return Err(Refusal::new(
                RefusalCode::InvalidArgs,
                "record a window",
                format!("nothing is being recorded under `{}`", id.0),
                "send `state: start` first and stop the id it answers with; a recording that \
                 was never started, or that already ended, has nothing to stop",
            ));
        };
        Ok(json!({
            "state": "stopped",
            "recording": id.0,
            "title": running.title,
            "into": running.into.display().to_string(),
            "as": running.written_by.close(),
        }))
    }
}

impl Drop for Recordings {
    /// A connection that closes mid-recording stops what it started.
    /// Leaving a thread grabbing frames of somebody's desktop after the
    /// caller has gone is the kind of thing this server exists to not do.
    fn drop(&mut self) {
        let running: Vec<RecordingId> = self.running.keys().copied().collect();
        for id in running {
            let _stopped = self.stop(id);
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
    use crate::platform::windows::target::Named;
    use windows::Win32::Foundation::HWND;

    /// A window with a handle no recording thread can draw from, which
    /// is what makes these tests about the bookkeeping alone.
    fn window(title: &str, handle: usize) -> Window {
        Window {
            named: Named {
                title: title.to_owned(),
                process: "test.exe".to_owned(),
            },
            bounds: super::super::geometry::Bounds::from_corners(0, 0, 32, 32).unwrap(),
            handle: HWND(std::ptr::without_provenance_mut(handle)),
        }
    }

    fn landed(answer: &Value) -> PathBuf {
        PathBuf::from(
            answer["into"]
                .as_str()
                .expect("a recording lands somewhere"),
        )
    }

    fn id_of(answer: &Value) -> RecordingId {
        RecordingId(answer["recording"].as_u64().expect("a start names its id"))
    }

    /// The two bookkeeping rules, which hold whatever this machine has
    /// installed: one recording per window, and nothing to stop until
    /// something started.
    #[test]
    fn one_window_records_once_and_stops_once() {
        let mut recordings = Recordings::new();
        let unstarted = recordings
            .stop(RecordingId(1))
            .expect_err("nothing is recording");
        assert_eq!(unstarted.as_error()["data"]["code"], "E_INVALID_ARGS");

        let started = recordings
            .start(&window("Calculator", 0x10))
            .expect("a recording starts even of a window that draws nothing");
        let into = landed(&started);
        assert!(into.is_dir(), "{} was not laid out", into.display());

        let twice = recordings
            .start(&window("Calculator", 0x10))
            .expect_err("two recordings of one window is not a thing `stop` can answer");
        assert_eq!(twice.as_error()["data"]["code"], "E_GATE_DENIED");

        let ended = recordings
            .stop(id_of(&started))
            .expect("the recording ends");
        assert_eq!(landed(&ended), into);
        assert!(matches!(ended["as"].as_str(), Some("mp4" | "frames")));
        // Stopping twice is the same refusal as stopping something that
        // never started, rather than a second file.
        assert!(recordings.stop(id_of(&started)).is_err());
        let _tidied = std::fs::remove_dir_all(&into);
    }

    /// The defect this id exists for: the title a recording started
    /// under is not the title the window carries when it is stopped.
    #[test]
    fn a_window_that_renames_itself_mid_recording_can_still_be_stopped() {
        let mut recordings = Recordings::new();
        let started = recordings.start(&window("a.txt — Notepad", 0x20)).unwrap();
        let ended = recordings
            .stop(id_of(&started))
            .expect("the id outlives the title");
        assert_eq!(ended["title"], "a.txt — Notepad");
        let _tidied = std::fs::remove_dir_all(landed(&started));
    }

    /// Sound is refused with the reason, not accepted and dropped, and
    /// the refusal happens before anything is started.
    #[test]
    fn asking_for_sound_is_refused_rather_than_silently_ignored() {
        let refusal = asked(&json!({ "state": "start", "audio": true }))
            .err()
            .expect("this server chooses no sound device");
        let error = refusal.as_error();
        assert_eq!(error["data"]["code"], "E_TOOL_UNAVAILABLE");
        assert!(
            error["data"]["recovery"]
                .as_str()
                .unwrap()
                .contains("without `audio`")
        );
    }

    /// A stop that names no recording is refused by name, because the
    /// alternative is stopping whichever one this desk happens to hold.
    #[test]
    fn a_stop_says_which_recording_it_ends() {
        assert!(matches!(
            asked(&json!({ "state": "start" })),
            Ok(Wanted::Start)
        ));
        assert!(matches!(
            asked(&json!({ "state": "stop", "recording": 7 })),
            Ok(Wanted::Stop(RecordingId(7)))
        ));
        let nameless = asked(&json!({ "state": "stop" })).err().unwrap();
        assert_eq!(nameless.as_error()["data"]["code"], "E_INVALID_ARGS");
        let neither = asked(&json!({ "state": "pause" })).err().unwrap();
        assert_eq!(neither.as_error()["data"]["code"], "E_INVALID_ARGS");
    }

    /// Two windows record independently, and neither stop reaches the
    /// other.
    #[test]
    fn two_windows_record_and_stop_independently() {
        let mut recordings = Recordings::new();
        let one = recordings.start(&window("Calculator", 0x30)).unwrap();
        let two = recordings.start(&window("a.txt — Notepad", 0x31)).unwrap();
        assert_ne!(landed(&one), landed(&two));
        assert!(recordings.stop(id_of(&one)).is_ok());
        assert!(recordings.stop(id_of(&one)).is_err());
        assert!(recordings.stop(id_of(&two)).is_ok());
        for gone in [landed(&one), landed(&two)] {
            let _tidied = std::fs::remove_dir_all(gone);
        }
    }

    /// Dropping the desk stops what it was running, so a connection that
    /// closes does not leave a thread grabbing frames of somebody's
    /// desktop. If the drop did not join the thread, this is the test
    /// that would hang rather than the one that would fail.
    #[test]
    fn dropping_the_table_stops_every_recording_it_held() {
        let landing = {
            let mut recordings = Recordings::new();
            let started = recordings
                .start(&window("a window only this test names", 0x40))
                .unwrap();
            assert_eq!(recordings.running.len(), 1);
            landed(&started)
        };
        assert!(landing.is_dir(), "{} was not laid out", landing.display());
        let _tidied = std::fs::remove_dir_all(&landing);
    }
}
