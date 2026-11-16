// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::super::capture;
use super::super::geometry::Bounds;
use crate::refusal::{Refusal, RefusalCode};

const FRAMES_A_SECOND: u64 = 10;

const FRAME_EVERY: std::time::Duration = std::time::Duration::from_millis(1_000 / FRAMES_A_SECOND);

const MOST_FRAMES: u64 = 6_000;

pub(super) const MOST_SECONDS: u64 = MOST_FRAMES / FRAMES_A_SECOND;

pub(super) const FFMPEG_FINISH_POLLS: u32 = 400;

pub(super) const POLL_EVERY: std::time::Duration = std::time::Duration::from_millis(50);

const NAMES_TRIED: u32 = 1_000;

pub(super) struct Sink {
    stopping: Arc<AtomicBool>,
    thread: std::thread::JoinHandle<Ended>,
    kind: &'static str,
}

pub(super) struct Closed {

    pub(super) kind: &'static str,
    pub(super) frames: u64,

    pub(super) cut_short: Option<Refusal>,
}

struct Ended {
    frames: u64,
    cut_short: Option<Refusal>,
}

enum Writer {

    Mp4 { child: Child, stdin: ChildStdin },

    Frames { into: PathBuf },
}

impl Sink {

    pub(super) fn open(
        handle: winsafe::HWND,
        bounds: Bounds,
        into: &Path,
    ) -> Result<Sink, Refusal> {
        let writer = match child::command("ffmpeg")
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

        let thread = std::thread::spawn(move || recorded(&handle, bounds, writer, &flag));
        Ok(Sink {
            stopping,
            thread,
            kind,
        })
    }

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

fn unstartable(why: &str) -> Refusal {
    Refusal::new(
        RefusalCode::ToolUnavailable,
        "record a window",
        format!("ffmpeg is on this machine and would not start: {why}"),
        "repair or remove the ffmpeg on this machine's PATH; without one, a recording is \
         written as a sequence of PNG frames",
    )
}

fn recorded(
    handle: &winsafe::HWND,
    bounds: Bounds,
    mut writer: Writer,
    stopping: &AtomicBool,
) -> Ended {
    let mut frames: u64 = 0;
    let mut cut_short = None;
    while !stopping.load(Ordering::Acquire) && frames < MOST_FRAMES {

        match capture::window(handle, bounds).and_then(|frame| writer.put(&frame, frames)) {
            Ok(()) => frames = frames.saturating_add(1),
            Err(refusal) => {
                cut_short = Some(refusal);
                break;
            }
        }

        std::thread::sleep(FRAME_EVERY);
    }
    writer.finish();
    Ended { frames, cut_short }
}

impl Writer {

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

                let _killed = child.kill();
                let _reaped = child.wait();
            }
            Writer::Frames { .. } => {}
        }
    }
}

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

    #[test]
    fn a_window_title_full_of_path_characters_still_lands_somewhere_safe() {
        let hostile = "../../etc/passwd — C:\\Windows\\*?";
        let into = somewhere(hostile, 1).expect("a hostile title is still a directory name");
        let name = into.file_name().unwrap_or_default().to_string_lossy();
        assert!(!name.contains('\\'), "{name}");
        assert!(!name.contains('/'), "{name}");
        assert!(!name.contains(".."), "{name}");
        assert!(into.starts_with(std::env::temp_dir()), "{}", into.display());

        let second = somewhere(hostile, 1).expect("a name in use is not a name reused");
        assert_ne!(second, into);
        for landed in [into, second] {
            let _tidied = std::fs::remove_dir_all(&landed);
        }
    }
}
