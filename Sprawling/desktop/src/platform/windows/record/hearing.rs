// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Stdio};

use super::sink::{FFMPEG_FINISH_POLLS, MOST_SECONDS, POLL_EVERY};
use crate::refusal::{Refusal, RefusalCode};

const SOUND_FILE: &str = "sound.wav";

const SAMPLE_RATE: &str = "16000";
const CHANNELS: &str = "1";

const SOUND_CARRIED_MOST: u64 = 4 * 1024 * 1024;

pub(super) struct Hearing {
    child: Child,
    stdin: ChildStdin,
    into: PathBuf,
}

pub(super) struct Heard {

    pub(super) into: PathBuf,

    pub(super) cut_short: Option<Refusal>,
}

impl Hearing {

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
