// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where one recording's bytes go, and what writes them: this package's
//! one thread captures every frame through `capture::window`, and hands
//! it to ffmpeg's stdin when this machine has ffmpeg, or writes it as a
//! PNG when it does not (`crates/desktop/Spec.lean` D7).
//!
//! **This file holds the only `std::thread::spawn` in the package.** It
//! is stopped by one flag and joined by [`Sink::close`], so a recording
//! cannot outlive the connection that started it.
//!
//! ffmpeg is never told which window to record. Told a title, it finds
//! a window on its own, by title alone, with a different pixel source
//! and none of this package's checks; handed frames on stdin, it can
//! only encode what `capture::window` already took from the window a
//! caller named. A frame that cannot be taken or written ends the
//! recording, and `stop` says how many frames there are and why it
//! ended early.
//!
//! The caller above decides *whether* a recording may begin; this file
//! decides *how* one is written and where it lands. Neither choice
//! reaches the other: `record` never names ffmpeg, and nothing here
//! knows that two recordings of one window are refused.
//!
//! Sound is not written here: it is a second ffmpeg's own input, in
//! `hearing`, and the two files are written side by side.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::super::capture;
use super::super::geometry::Bounds;
use crate::refusal::{Refusal, RefusalCode};

/// How many frames a second both writers aim for. `PrintWindow` costs
/// what it costs, and ten frames a second is enough to see one
/// interaction happen (`crates/desktop/Spec.lean` §14).
const FRAMES_A_SECOND: u64 = 10;

/// The shortest gap between two frames of the frame-sequence path,
/// derived from the rate rather than written beside it.
const FRAME_EVERY: std::time::Duration = std::time::Duration::from_millis(1_000 / FRAMES_A_SECOND);

/// The most frames one recording writes: ten minutes at the rate above.
///
/// **A recording this server started has to end even if nobody stops
/// it.** The caller may close its connection, forget the id, or ask for
/// a window nobody touches again; without this the thread would keep
/// writing PNGs of somebody's desktop until the disk filled. Both
/// writers are held to the same number, so "how long is the longest
/// recording" has one answer whichever one this machine ran.
const MOST_FRAMES: u64 = 6_000;

/// The same ceiling as the seconds ffmpeg is given, which is the only
/// form ffmpeg accepts it in.
pub(super) const MOST_SECONDS: u64 = MOST_FRAMES / FRAMES_A_SECOND;

/// How long [`Sink::close`] waits for ffmpeg to finish writing the
/// file's index, counted in polls rather than against a clock.
///
/// Counted rather than timed on purpose: this package reads no clock at
/// all, which is the same rule the city holds (`clippy.toml`), and a
/// bounded count of sleeps bounds the wait just as well. Four hundred
/// polls a twentieth of a second apart is about twenty seconds.
pub(super) const FFMPEG_FINISH_POLLS: u32 = 400;

/// How long one of those polls sleeps.
pub(super) const POLL_EVERY: std::time::Duration = std::time::Duration::from_millis(50);

/// How many names are tried before this gives up looking for a free
/// one. A machine with a thousand un-cleared recordings of one window
/// has a housekeeping problem, and silently overwriting the thousandth
/// would not be the fix.
const NAMES_TRIED: u32 = 1_000;

/// One recording being written: the thread that writes it, the flag that
/// stops it, and which of the two kinds of file it is.
pub(super) struct Sink {
    stopping: Arc<AtomicBool>,
    thread: std::thread::JoinHandle<Ended>,
    kind: &'static str,
}

/// What a recording came to once it stopped.
pub(super) struct Closed {
    /// `mp4` or `frames`.
    pub(super) kind: &'static str,
    pub(super) frames: u64,
    /// Why the recording stopped before anyone asked it to, if it did.
    pub(super) cut_short: Option<Refusal>,
}

/// What the writing thread hands back when it ends.
struct Ended {
    frames: u64,
    cut_short: Option<Refusal>,
}

/// Where the frames go.
enum Writer {
    /// ffmpeg, reading raw frames from the stdin this holds. Closing
    /// that stdin is what asks it to finish: it writes the index an mp4
    /// needs once its input ends, and a killed ffmpeg leaves a file
    /// nothing plays.
    Mp4 { child: Child, stdin: ChildStdin },
    /// One PNG per frame, numbered, in this directory.
    Frames { into: PathBuf },
}

impl Sink {
    /// Starts writing one window into `into`.
    ///
    /// # Errors
    /// Refuses when ffmpeg is on this machine and will not start. A
    /// machine without ffmpeg gets the frame sequence instead.
    pub(super) fn open(
        handle: winsafe::HWND,
        bounds: Bounds,
        into: &Path,
    ) -> Result<Sink, Refusal> {
        let writer = match Command::new("ffmpeg")
            .args(command_line(bounds, into))
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(mut child) => match child.stdin.take() {
                Some(stdin) => Writer::Mp4 { child, stdin },
                None => {
                    let _killed = child.kill();
                    let _reaped = child.wait();
                    return Err(unstartable("it gave no stdin to write frames into"));
                }
            },
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Writer::Frames {
                into: into.to_path_buf(),
            },
            Err(err) => return Err(unstartable(&err.to_string())),
        };
        let kind = match writer {
            Writer::Mp4 { .. } => "mp4",
            Writer::Frames { .. } => "frames",
        };
        let stopping = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stopping);
        // The handle moves to the thread: a `winsafe::HWND` is `Send`,
        // and Win32 documents the drawing calls this thread makes as
        // usable from any thread with the window's handle.
        let thread = std::thread::spawn(move || recorded(&handle, bounds, writer, &flag));
        Ok(Sink {
            stopping,
            thread,
            kind,
        })
    }

    /// Stops writing, and says what was written.
    pub(super) fn close(self) -> Closed {
        self.stopping.store(true, Ordering::Release);
        let Ended { frames, cut_short } = self.thread.join().unwrap_or_else(|_panicked| Ended {
            frames: 0,
            cut_short: Some(Refusal::new(
                RefusalCode::ToolUnavailable,
                "record a window",
                "the thread writing this recording stopped unexpectedly",
                "look at what landed before relying on it, and record again",
            )),
        });
        Closed {
            kind: self.kind,
            frames,
            cut_short,
        }
    }
}

/// The refusal for an ffmpeg that is installed and will not start.
fn unstartable(why: &str) -> Refusal {
    Refusal::new(
        RefusalCode::ToolUnavailable,
        "record a window",
        format!("ffmpeg is on this machine and would not start: {why}"),
        "repair or remove the ffmpeg on this machine's PATH; without one, a recording is \
         written as a sequence of PNG frames",
    )
}

/// The writing thread: one frame every `FRAME_EVERY` until it is told to
/// stop, reaches `MOST_FRAMES`, or cannot take or write a frame.
fn recorded(
    handle: &winsafe::HWND,
    bounds: Bounds,
    mut writer: Writer,
    stopping: &AtomicBool,
) -> Ended {
    let mut frames: u64 = 0;
    let mut cut_short = None;
    while !stopping.load(Ordering::Acquire) && frames < MOST_FRAMES {
        // The frame is taken at the size the recording started with,
        // which is the size ffmpeg was told to expect.
        match capture::window(handle, bounds).and_then(|frame| writer.put(&frame, frames)) {
            Ok(()) => frames = frames.saturating_add(1),
            Err(refusal) => {
                cut_short = Some(refusal);
                break;
            }
        }
        // A fixed rest between frames rather than a deadline measured
        // from the frame's start: this package reads no clock. What that
        // costs is stated rather than hidden — a window that is slow to
        // draw yields a sparser recording, so `FRAME_EVERY` is the
        // shortest gap between frames and not a promised rate.
        std::thread::sleep(FRAME_EVERY);
    }
    writer.finish();
    Ended { frames, cut_short }
}

impl Writer {
    /// Writes the `nth` frame.
    fn put(&mut self, frame: &image::RgbaImage, nth: u64) -> Result<(), Refusal> {
        match self {
            Writer::Mp4 { stdin, .. } => stdin.write_all(frame.as_raw()).map_err(|err| {
                Refusal::new(
                    RefusalCode::ToolUnavailable,
                    "record a window",
                    format!("ffmpeg stopped taking frames: {err}"),
                    "look at what landed; ffmpeg ended the recording before it was stopped",
                )
            }),
            Writer::Frames { into } => frame
                .save(into.join(format!("frame-{nth:06}.png")))
                .map_err(|err| {
                    Refusal::new(
                        RefusalCode::ToolUnavailable,
                        "record a window",
                        format!("a frame could not be written: {err}"),
                        "free space in this machine's temporary directory, then record again",
                    )
                }),
        }
    }

    /// Ends the file: ffmpeg's input is closed and it is given
    /// `FFMPEG_FINISH_POLLS` polls to write its index and exit.
    fn finish(self) {
        match self {
            Writer::Mp4 { mut child, stdin } => {
                drop(stdin);
                for _poll in 0..FFMPEG_FINISH_POLLS {
                    match child.try_wait() {
                        Ok(Some(_ended)) => return,
                        Ok(None) => std::thread::sleep(POLL_EVERY),
                        Err(_unknowable) => break,
                    }
                }
                // It was given its end of input and twenty seconds. What
                // is left is a partial file rather than a process nobody
                // can stop.
                let _killed = child.kill();
                let _reaped = child.wait();
            }
            Writer::Frames { .. } => {}
        }
    }
}

/// Where a recording of this window goes.
///
/// Not into the city and not into the person's own folders: the scope
/// file says which windows this server may touch and says nothing about
/// where it may write, so the answer is the place the operating system
/// keeps things nobody promised to keep (`crates/desktop/Spec.lean` §14).
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

/// The arguments ffmpeg is started with: raw RGBA frames of the
/// recording's size on stdin, one mp4 out.
///
/// The `pad` filter rounds an odd width or height up to even, which the
/// yuv420p an ordinary player reads cannot do without.
fn command_line(bounds: Bounds, into: &Path) -> Vec<String> {
    let mut line: Vec<String> = [
        "-hide_banner",
        "-loglevel",
        "error",
        "-f",
        "rawvideo",
        "-pixel_format",
        "rgba",
        "-video_size",
    ]
    .map(str::to_owned)
    .to_vec();
    line.push(format!("{}x{}", bounds.width(), bounds.height()));
    line.extend(["-framerate".to_owned(), FRAMES_A_SECOND.to_string()]);
    line.extend(["-i".to_owned(), "-".to_owned()]);
    // ffmpeg stops itself at the same ceiling the frame thread holds,
    // and writes the index for what it recorded.
    line.extend(["-t".to_owned(), MOST_SECONDS.to_string()]);
    line.extend(
        [
            "-vf",
            "pad=ceil(iw/2)*2:ceil(ih/2)*2",
            "-pix_fmt",
            "yuv420p",
            "-y",
        ]
        .map(str::to_owned),
    );
    line.push(into.join("recording.mp4").display().to_string());
    line
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

    /// ffmpeg is handed frames this package captured through
    /// `capture::window`, never a title it would look up again on its
    /// own, where another window with the same title could answer.
    #[test]
    fn a_recording_hands_ffmpeg_frames_rather_than_a_window_title() {
        let bounds = Bounds::from_corners(0, 0, 640, 480).unwrap();
        let line = command_line(bounds, Path::new("recorded"));
        let adjacent = |first: &str, second: &str| {
            line.windows(2)
                .any(|pair| pair[0] == first && pair[1] == second)
        };
        assert!(adjacent("-f", "rawvideo"), "{line:?}");
        assert!(adjacent("-pixel_format", "rgba"), "{line:?}");
        assert!(adjacent("-video_size", "640x480"), "{line:?}");
        assert!(adjacent("-i", "-"), "{line:?}");
        assert!(
            line.iter()
                .all(|argument| !argument.starts_with("title=") && argument != "gdigrab"),
            "{line:?}"
        );
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
