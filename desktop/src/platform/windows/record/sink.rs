// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where one recording's bytes go, and what writes them: ffmpeg when
//! this machine has it, and this package's one thread when it does not.
//!
//! **This file holds the only `std::thread::spawn` in the package**, and
//! it is here because the frame-sequence path is the only thing that
//! has to happen while the read loop is waiting for the next request.
//! It is stopped by one flag and joined by [`Sink::close`], so a
//! recording cannot outlive the connection that started it.
//!
//! The caller above decides *whether* a recording may begin; this file
//! decides *how* one is written and where it lands. Neither choice
//! reaches the other: `record` never names ffmpeg, and nothing here
//! knows that two recordings of one window are refused.
//!
//! Sound is not written here either, because nothing in this server
//! chooses a capture device; the refusal for it belongs to the caller,
//! beside the other reasons a recording does not begin.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use windows::Win32::Foundation::HWND;

use super::super::geometry::Bounds;
use crate::refusal::{Refusal, RefusalCode};

/// The shortest gap between two frames of the frame-sequence path.
/// `PrintWindow` costs what it costs, and ten frames a second is enough
/// to see one interaction happen (desktop-SPEC.md §14).
const FRAME_EVERY: std::time::Duration = std::time::Duration::from_millis(100);

/// How long [`Sink::close`] waits for ffmpeg to finish writing the
/// file's index, counted in polls rather than against a clock.
///
/// Counted rather than timed on purpose: this package reads no clock at
/// all, which is the same rule the city holds (`clippy.toml`), and a
/// bounded count of sleeps bounds the wait just as well. Four hundred
/// polls a twentieth of a second apart is about twenty seconds.
const FFMPEG_FINISH_POLLS: u32 = 400;

/// How long one of those polls sleeps.
const POLL_EVERY: std::time::Duration = std::time::Duration::from_millis(50);

/// How many names are tried before this gives up looking for a free
/// one. A machine with a thousand un-cleared recordings of one window
/// has a housekeeping problem, and silently overwriting the thousandth
/// would not be the fix.
const NAMES_TRIED: u32 = 1_000;

/// What is writing one recording.
pub(super) enum Sink {
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

impl Sink {
    /// Starts writing one window into `into`.
    ///
    /// Infallible by construction: a machine without ffmpeg gets the
    /// frame sequence, and a window that draws nothing yields a shorter
    /// sequence rather than an error. What can fail is laying out the
    /// directory, and that has already happened by the time this is
    /// called.
    pub(super) fn open(window: &str, handle: HWND, bounds: Bounds, into: &Path) -> Sink {
        match ffmpeg(window, into) {
            Some(child) => Sink::Ffmpeg { child },
            None => frames(handle, bounds, into),
        }
    }

    /// Stops writing, and says which of the two kinds of file was
    /// written.
    pub(super) fn close(self) -> &'static str {
        match self {
            Sink::Ffmpeg { child } => {
                finish(child);
                "mp4"
            }
            Sink::Frames { stopping, thread } => {
                stopping.store(true, Ordering::Release);
                let _frames = thread.join();
                "frames"
            }
        }
    }
}

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
///
/// # Errors
/// Refuses a temporary directory that cannot be written to, and a
/// window whose thousand previous recordings are all still there.
pub(super) fn somewhere(window: &str, nth: u64) -> Result<PathBuf, Refusal> {
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
fn frames(handle: HWND, bounds: Bounds, into: &Path) -> Sink {
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
            if let Ok(pixels) = super::super::capture::window(handle, bounds) {
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
    Sink::Frames { stopping, thread }
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
