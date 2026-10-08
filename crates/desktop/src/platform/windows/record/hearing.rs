// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The sound of one recording: a second ffmpeg reading the one
//! DirectShow device the scope file names, writing a 16 kHz mono wav
//! into the recording's own directory (`crates/desktop/Spec.lean` D11).
//!
//! **This server never chooses the device.** The name arrives from
//! `scope::Admitted::sound`, which is the operator's word for which
//! sound source a run may hear; a device nobody named is a permission
//! nobody gave, and the gate has already refused it before this file is
//! reached.
//!
//! The sound is its own process and its own file, apart from the
//! frames: the frames come in through the first ffmpeg's stdin and the
//! sound is the second ffmpeg's own input, and putting both in one
//! process would mean aligning two clocks. What reaches the city is the
//! file, as an audio block of the `stop` answer when it is short enough
//! to carry (section 12.13).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Stdio};

use super::sink::{FFMPEG_FINISH_POLLS, MOST_SECONDS, POLL_EVERY};
use crate::refusal::{Refusal, RefusalCode};

/// The file the sound lands in, beside the frames.
const SOUND_FILE: &str = "sound.wav";

/// What the city's `transcribe` needs of speech: 16 kHz, one channel.
/// More is only more bytes (`crates/desktop/Spec.lean` section 14).
const SAMPLE_RATE: &str = "16000";
const CHANNELS: &str = "1";

/// The most sound one `stop` answer carries: base64 makes it a third
/// longer, and the whole line stays inside the city's MCP line ceiling
/// of 8 MiB (`crates/desktop/Spec.lean` D13 and section 14).
const SOUND_CARRIED_MOST: u64 = 4 * 1024 * 1024;

/// The sound's ffmpeg, while it records.
pub(super) struct Hearing {
    child: Child,
    stdin: ChildStdin,
    into: PathBuf,
}

/// What a recording's sound came to once it stopped.
pub(super) struct Heard {
    /// The wav, which may be short or absent when `cut_short` says so.
    pub(super) into: PathBuf,
    /// Why the sound stopped before the recording did, if it did.
    pub(super) cut_short: Option<Refusal>,
}

impl Hearing {
    /// Starts hearing `device` into `into`.
    ///
    /// # Errors
    /// Refuses a machine with no ffmpeg — the frame sequence a recording
    /// falls back to has no sound — and an ffmpeg that will not start.
    pub(super) fn open(device: &str, into: &Path) -> Result<Hearing, Refusal> {
        let spawned = child::command("ffmpeg")
            .args(arguments(device, into))
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        let mut child = match spawned {
            Ok(child) => child,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(unheard(
                    "this machine has no ffmpeg, and the sound of a recording is recorded by one"
                        .to_owned(),
                    "install ffmpeg (`sprawling doctor` lists it), or record without `audio`",
                ));
            }
            Err(err) => {
                return Err(unheard(
                    format!("ffmpeg would not start to record sound: {err}"),
                    "repair the ffmpeg on this machine's PATH, or record without `audio`",
                ));
            }
        };
        let Some(stdin) = child.stdin.take() else {
            let _killed = child.kill();
            let _reaped = child.wait();
            return Err(unheard(
                "ffmpeg gave no stdin to stop the sound with".to_owned(),
                "record without `audio`",
            ));
        };
        Ok(Hearing {
            child,
            stdin,
            into: into.join(SOUND_FILE),
        })
    }

    /// Stops hearing: `q` on ffmpeg's stdin, which is its own key for
    /// "finish", so it writes the wav's lengths before it exits. It is
    /// given `FFMPEG_FINISH_POLLS` polls, and stopped after them.
    pub(super) fn close(self) -> Heard {
        let Hearing {
            mut child,
            mut stdin,
            into,
        } = self;
        let asked = stdin.write_all(b"q\n").and_then(|()| stdin.flush());
        drop(stdin);
        let mut ended = None;
        for _poll in 0..FFMPEG_FINISH_POLLS {
            match child.try_wait() {
                Ok(Some(status)) => {
                    ended = Some(status);
                    break;
                }
                Ok(None) => std::thread::sleep(POLL_EVERY),
                Err(_unknowable) => break,
            }
        }
        let cut_short = match (asked, ended) {
            (Ok(()), Some(_finished)) => None,
            (Err(_gone), Some(status)) if status.success() => None,
            (Err(_gone), Some(status)) => Some(unheard(
                format!("the sound stopped before the recording did: ffmpeg exited with {status}"),
                "look at what landed; the sound device may be in use by another program, or gone",
            )),
            (_, None) => {
                let _killed = child.kill();
                let _reaped = child.wait();
                Some(unheard(
                    "ffmpeg did not finish writing the sound and was stopped".to_owned(),
                    "look at what landed; the wav may end early",
                ))
            }
        };
        Heard { into, cut_short }
    }
}

impl Heard {
    /// The wav's bytes, when it is there and short enough for one
    /// answer to carry, or the sentence that says why it is not.
    pub(super) fn carried(&self) -> Result<Vec<u8>, String> {
        let shown = self.into.display();
        let size = std::fs::metadata(&self.into)
            .map_err(|err| format!("{shown} could not be read: {err}"))?
            .len();
        if size > SOUND_CARRIED_MOST {
            return Err(format!(
                "the sound is {size} bytes, more than the {SOUND_CARRIED_MOST} one answer \
                 carries; it is at {shown}"
            ));
        }
        std::fs::read(&self.into).map_err(|err| format!("{shown} could not be read: {err}"))
    }
}

fn unheard(subject: String, recovery: &str) -> Refusal {
    Refusal::new(
        RefusalCode::ToolUnavailable,
        "record a window",
        subject,
        recovery,
    )
}

/// The arguments the sound's ffmpeg is started with: the named
/// DirectShow audio input, one channel at 16 kHz, stopped at the same
/// ceiling the frames are, into `sound.wav`.
pub(super) fn arguments(device: &str, into: &Path) -> Vec<String> {
    let mut line: Vec<String> = ["-hide_banner", "-loglevel", "error", "-f", "dshow", "-i"]
        .map(str::to_owned)
        .to_vec();
    line.push(format!("audio={device}"));
    line.extend(
        ["-ac", CHANNELS, "-ar", SAMPLE_RATE]
            .map(str::to_owned)
            .to_vec(),
    );
    line.extend(["-t".to_owned(), MOST_SECONDS.to_string(), "-y".to_owned()]);
    line.push(into.join(SOUND_FILE).display().to_string());
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

    /// ffmpeg is told the device the scope file names, as DirectShow
    /// spells an audio input, and writes the 16 kHz mono wav the city's
    /// `transcribe` takes, into the recording's own directory.
    #[test]
    fn the_device_the_scope_file_names_is_the_device_ffmpeg_hears() {
        let line = arguments("Microphone (USB Audio)", Path::new("recorded"));
        let adjacent = |first: &str, second: &str| {
            line.windows(2)
                .any(|pair| pair[0] == first && pair[1] == second)
        };
        assert!(adjacent("-f", "dshow"), "{line:?}");
        assert!(adjacent("-i", "audio=Microphone (USB Audio)"), "{line:?}");
        assert!(adjacent("-ac", "1"), "{line:?}");
        assert!(adjacent("-ar", "16000"), "{line:?}");
        assert_eq!(
            line.last().map(String::as_str),
            Some(
                Path::new("recorded")
                    .join("sound.wav")
                    .display()
                    .to_string()
                    .as_str()
            ),
            "{line:?}"
        );
    }

    /// A wav too long for one answer is left where it is, and the
    /// sentence says how long and where, so a caller does not read a
    /// missing audio block as silence.
    #[test]
    fn a_sound_too_long_to_carry_is_named_rather_than_carried() {
        let dir = tempfile::tempdir().unwrap();
        let into = dir.path().join(SOUND_FILE);
        let long = std::fs::File::create(&into).unwrap();
        long.set_len(SOUND_CARRIED_MOST + 1).unwrap();
        let heard = Heard {
            into,
            cut_short: None,
        };
        let why = heard.carried().expect_err("too long to carry");
        assert!(why.contains(&SOUND_CARRIED_MOST.to_string()), "{why}");
        let short = Heard {
            into: dir.path().join("short.wav"),
            cut_short: None,
        };
        std::fs::write(&short.into, b"RIFF").unwrap();
        assert_eq!(short.carried(), Ok(b"RIFF".to_vec()));
    }
}
