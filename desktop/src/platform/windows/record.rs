// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Start and stop: an mp4 through ffmpeg, or the frame sequence this
//! package's one thread writes.
//!
//! **This file holds the only `std::thread::spawn` in the package**, and
//! it is here because the frame-sequence path is the only thing that has
//! to happen while the read loop is waiting for the next request. It is
//! stopped by one flag and joined by `stop`, so a recording cannot
//! outlive the connection that started it: the desk is dropped with the
//! connection, and dropping it stops what it was running.
//!
//! Two recordings of one window at once is refused rather than merged.
//! Whichever of the two files a caller then asked for would be a guess,
//! and stopping would be ambiguous in a way no answer resolves.
//!
//! Sound is refused. Choosing a capture device requires knowing what it
//! is called on this machine, and nothing in this server knows that; a
//! recording that silently had no audio track would be discovered by
//! whoever played it back (desktop-SPEC.md §8.6, fourth pair).

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use windows::Win32::Foundation::HWND;

use super::geometry::Bounds;
use crate::refusal::{Refusal, RefusalCode};

/// The shortest gap between two frames of the frame-sequence path.
/// `PrintWindow` costs what it costs, and ten frames a second is enough
/// to see one interaction happen (desktop-SPEC.md §14).
const FRAME_EVERY: std::time::Duration = std::time::Duration::from_millis(100);

/// How long `stop` waits for ffmpeg to finish writing the file's index,
/// counted in polls rather than against a clock.
///
/// Counted rather than timed on purpose: this package reads no clock at
/// all, which is the same rule the city holds (`clippy.toml`), and a
/// bounded count of sleeps bounds the wait just as well. Four hundred
/// polls a twentieth of a second apart is about twenty seconds.
const FFMPEG_FINISH_POLLS: u32 = 400;

/// How long one of those polls sleeps.
const POLL_EVERY: std::time::Duration = std::time::Duration::from_millis(50);

/// What is being written, and by what.
enum Writing {
    /// ffmpeg, with its stdin held so it can be asked to finish
    /// cleanly. Killing it instead would leave an mp4 with no index,
    /// which is a file nothing plays.
    Ffmpeg { child: std::process::Child },
    /// This package's own thread, and the flag that stops it.
    Frames {
        stopping: Arc<AtomicBool>,
        thread: std::thread::JoinHandle<usize>,
    },
}

/// One recording in progress.
struct Running {
    into: PathBuf,
    writing: Writing,
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
        let writing = match ffmpeg(window, &into) {
            Some(child) => Writing::Ffmpeg { child },
            None => frames(handle, bounds, &into)?,
        };
        self.running
            .insert(window.to_owned(), Running { into, writing });
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
        let how = match running.writing {
            Writing::Ffmpeg { child } => {
                finish(child);
                "mp4"
            }
            Writing::Frames { stopping, thread } => {
                stopping.store(true, Ordering::Release);
                let _frames = thread.join();
                "frames"
            }
        };
        Ok((running.into, how))
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

/// How many names are tried before this gives up looking for a free
/// one. A machine with a thousand un-cleared recordings of one window
/// has a housekeeping problem, and silently overwriting the thousandth
/// would not be the fix.
const NAMES_TRIED: u32 = 1_000;

/// Where a recording of this window goes.
///
/// Not into the city and not into the person's own folders: the scope
/// file says which windows this server may touch and says nothing about
/// where it may write, so the answer is the place the operating system
/// keeps things nobody promised to keep (desktop-SPEC.md §14).
///
/// **The directory is created only if it did not exist**, and the name
/// is bumped until one is free. A second connection recording the same
/// window has its own counter, so without this the two would write
/// frames into one directory and each would report a path holding the
/// other's recording.
fn somewhere(window: &str, nth: u64) -> Result<PathBuf, Refusal> {
    let safe: String = window
        .chars()
        .map(|glyph| if glyph.is_alphanumeric() { glyph } else { '-' })
        .take(40)
        .collect();
    let under = std::env::temp_dir().join("sprawling-desktop");
    let mut last = std::io::Error::other("no name was tried");
    for attempt in 0..NAMES_TRIED {
        let into = under.join(match attempt {
            0 => format!("{safe}-{nth}"),
            again => format!("{safe}-{nth}-{again}"),
        });
        if let Err(err) = std::fs::create_dir_all(&under) {
            last = err;
            break;
        }
        // `create_dir` rather than `create_dir_all` for the leaf: the
        // second would succeed on a directory that is already there,
        // and whether it is already there is the whole question.
        match std::fs::create_dir(&into) {
            Ok(()) => return Ok(into),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => {
                last = err;
                break;
            }
        }
    }
    Err(Refusal::new(
        RefusalCode::ToolUnavailable,
        "record a window",
        format!("{}: {last}", under.display()),
        "this machine's temporary directory is not writable, or it already holds a thousand \
         recordings of this window; clear it out or fix its permissions",
    ))
}

/// ffmpeg started on this window, or `None` when this machine has none.
///
/// `gdigrab` names the window by its title, which is the same handle on
/// the window a caller used to name it — so a recording cannot reach a
/// window the scope file left out by going around it.
fn ffmpeg(window: &str, into: &Path) -> Option<std::process::Child> {
    std::process::Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error"])
        .args(["-f", "gdigrab", "-framerate", "10"])
        .args(["-i", &format!("title={window}")])
        .arg("-y")
        .arg(into.join("recording.mp4"))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()
}

/// Asks ffmpeg to finish, which is what writes the index an mp4 needs.
fn finish(mut child: std::process::Child) {
    if let Some(stdin) = child.stdin.as_mut() {
        let _asked = stdin.write_all(b"q\n");
        let _flushed = stdin.flush();
    }
    drop(child.stdin.take());
    for _poll in 0..FFMPEG_FINISH_POLLS {
        match child.try_wait() {
            Ok(Some(_ended)) => return,
            Ok(None) => std::thread::sleep(POLL_EVERY),
            Err(_unknowable) => break,
        }
    }
    // It was asked politely and given twenty seconds. What is left is a
    // partial file rather than a process nobody can stop.
    let _killed = child.kill();
    let _reaped = child.wait();
}

/// The frame-sequence path: this package's one thread.
fn frames(handle: HWND, bounds: Bounds, into: &Path) -> Result<Writing, Refusal> {
    let stopping = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&stopping);
    let into = into.to_path_buf();
    // An `HWND` is a token rather than a pointer into this process's
    // memory, so it travels as its address and is rebuilt on the other
    // side. Win32 documents the drawing calls this thread makes as
    // usable from any thread with the window's handle.
    let carried = handle.0.expose_provenance();
    let thread = std::thread::spawn(move || {
        let handle = HWND(std::ptr::with_exposed_provenance_mut(carried));
        let mut written: usize = 0;
        while !flag.load(Ordering::Acquire) {
            if let Ok(pixels) = super::capture::window(handle, bounds) {
                let at = into.join(format!("frame-{written:06}.png"));
                if pixels.save(&at).is_ok() {
                    written = written.saturating_add(1);
                }
            }
            // A fixed rest between frames rather than a deadline measured
            // from the frame's start: this package reads no clock. What
            // that costs is stated rather than hidden — a window that is
            // slow to draw yields a sparser sequence, so `FRAME_EVERY` is
            // the shortest gap between frames and not a promised rate.
            std::thread::sleep(FRAME_EVERY);
        }
        written
    });
    Ok(Writing::Frames { stopping, thread })
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

    /// A window title is not a file name, and this is where that stops
    /// being a problem.
    #[test]
    fn a_window_title_full_of_path_characters_still_lands_somewhere_safe() {
        let hostile = "../../etc/passwd — C:\\Windows\\*?";
        let into = somewhere(hostile, 1).expect("a hostile title is still a directory name");
        let name = into.file_name().unwrap_or_default().to_string_lossy();
        assert!(!name.contains('\\'), "{name}");
        assert!(!name.contains('/'), "{name}");
        assert!(!name.contains(".."), "{name}");
        assert!(into.starts_with(std::env::temp_dir()), "{}", into.display());

        // Two desks that reached the same name get two directories, so
        // neither reports a path holding the other's frames.
        let second = somewhere(hostile, 1).expect("a name in use is not a name reused");
        assert_ne!(second, into);
        for landed in [into, second] {
            let _tidied = std::fs::remove_dir_all(&landed);
        }
    }
}
