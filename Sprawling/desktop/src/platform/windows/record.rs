// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod hearing;
mod sink;

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde_json::{Value, json};

use super::enumerate::Window;
use crate::answer::Answer;
use crate::refusal::{Refusal, RefusalCode};
use hearing::Hearing;
use sink::{Sink, somewhere};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RecordingId(u64);

pub(crate) enum Wanted {
    Start(Sound),

    Stop(RecordingId),
}

pub(crate) enum Sound {
    Silent,

    Asked,
}

pub(crate) fn asked(arguments: &Value) -> Result<Wanted, Refusal> {
    let state = arguments
        .get("state")
        .and_then(Value::as_str)
        .ok_or_else(|| named_neither("the call says neither start nor stop".to_owned()))?;
    match state {
        "start" => match arguments.get("audio") {
            None | Some(Value::Bool(false)) => Ok(Wanted::Start(Sound::Silent)),
            Some(Value::Bool(true)) => Ok(Wanted::Start(Sound::Asked)),

            Some(other) => Err(Refusal::new(
                RefusalCode::InvalidArgs,
                "record a window",
                format!("`audio` is {other}, and it is true or false"),
                "send `audio: true` to hear the sound device the scope file names, or leave it out",
            )),
        },
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

struct Running {
    of: usize,
    title: String,
    into: PathBuf,
    written_by: Sink,
    heard_by: Option<Hearing>,
}

#[derive(Default)]
pub(crate) struct Recordings {
    running: BTreeMap<RecordingId, Running>,

    begun: u64,
}

impl Recordings {
    pub(crate) fn new() -> Recordings {
        Recordings {
            running: BTreeMap::new(),
            begun: 0,
        }
    }

    pub(crate) fn start(&mut self, window: Window, device: Option<&str>) -> Result<Value, Refusal> {
        let of = window.handle.ptr().addr();
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
        let title = window.named.title;

        let heard_by = device
            .map(|device| Hearing::open(device, &into))
            .transpose()?;
        let written_by = match Sink::open(window.handle, window.bounds, &into) {
            Ok(written_by) => written_by,
            Err(refusal) => {
                let _abandoned = heard_by.map(Hearing::close);
                return Err(refusal);
            }
        };
        let answer = json!({
            "state": "started",
            "recording": id.0,
            "title": title,
            "into": into.display().to_string(),
            "sound": device,
        });
        self.running.insert(
            id,
            Running {
                of,
                title,
                into,
                written_by,
                heard_by,
            },
        );
        Ok(answer)
    }

    pub(crate) fn stop(&mut self, id: RecordingId) -> Result<Answer, Refusal> {
        let Some(running) = self.running.remove(&id) else {
            return Err(Refusal::new(
                RefusalCode::InvalidArgs,
                "record a window",
                format!("nothing is being recorded under `{}`", id.0),
                "send `state: start` first and stop the id it answers with; a recording that \
                 was never started, or that already ended, has nothing to stop",
            ));
        };
        let closed = running.written_by.close();
        let mut answer = json!({
            "state": "stopped",
            "recording": id.0,
            "title": running.title,
            "into": running.into.display().to_string(),
            "as": closed.kind,
            "frames": closed.frames,
        });

        if let (Some(object), Some(early)) = (answer.as_object_mut(), closed.cut_short) {
            object.insert("ended_early".to_owned(), json!(early.summary()));
        }
        let Some(heard) = running.heard_by.map(Hearing::close) else {
            return Ok(Answer::facts(&answer));
        };
        let carried = heard.carried();
        if let Some(object) = answer.as_object_mut() {
            object.insert("sound".to_owned(), json!(heard.into.display().to_string()));
            if let Some(early) = &heard.cut_short {
                object.insert("sound_ended_early".to_owned(), json!(early.summary()));
            }
            if let Err(why) = &carried {
                object.insert("sound_left_out".to_owned(), json!(why));
            }
        }
        Ok(match carried {
            Ok(bytes) => Answer::sound(bytes, "audio/wav", &answer),
            Err(_said_above) => Answer::facts(&answer),
        })
    }
}

impl Drop for Recordings {
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

    fn window(title: &str, handle: usize) -> Window {
        Window {
            named: Named {
                title: title.to_owned(),
                process: "test.exe".to_owned(),
            },
            bounds: super::super::geometry::Bounds::from_corners(0, 0, 32, 32).unwrap(),

            handle: desktop_ffi::fixture::made_up(handle),
        }
    }

    impl Recordings {
        fn start_silent(&mut self, window: Window) -> Result<Value, Refusal> {
            self.start(window, None)
        }

        fn stop_facts(&mut self, id: RecordingId) -> Result<Value, Refusal> {
            self.stop(id).map(|answer| {
                let result = answer.as_result();
                let text = result["content"][0]["text"].as_str().unwrap().to_owned();
                serde_json::from_str(&text).unwrap()
            })
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

    #[test]
    fn one_window_records_once_and_stops_once() {
        let mut recordings = Recordings::new();
        let unstarted = recordings
            .stop(RecordingId(1))
            .expect_err("nothing is recording");
        assert_eq!(unstarted.as_error()["data"]["code"], "E_INVALID_ARGS");

        let started = recordings
            .start_silent(window("Calculator", 0x10))
            .expect("a recording starts even of a window that draws nothing");
        let into = landed(&started);
        assert!(into.is_dir(), "{} was not laid out", into.display());

        let twice = recordings
            .start_silent(window("Calculator", 0x10))
            .expect_err("two recordings of one window is not a thing `stop` can answer");
        assert_eq!(twice.as_error()["data"]["code"], "E_GATE_DENIED");

        let ended = recordings
            .stop_facts(id_of(&started))
            .expect("the recording ends");
        assert_eq!(landed(&ended), into);
        assert!(matches!(ended["as"].as_str(), Some("mp4" | "frames")));

        assert!(recordings.stop(id_of(&started)).is_err());
        let _tidied = std::fs::remove_dir_all(&into);
    }

    #[test]
    fn a_window_that_renames_itself_mid_recording_can_still_be_stopped() {
        let mut recordings = Recordings::new();
        let started = recordings
            .start_silent(window("a.txt — Notepad", 0x20))
            .unwrap();
        let ended = recordings
            .stop_facts(id_of(&started))
            .expect("the id outlives the title");
        assert_eq!(ended["title"], "a.txt — Notepad");
        let _tidied = std::fs::remove_dir_all(landed(&started));
    }

    #[test]
    fn audio_is_true_or_false_and_nothing_else() {
        assert!(matches!(
            asked(&json!({ "state": "start", "audio": true })),
            Ok(Wanted::Start(Sound::Asked))
        ));
        assert!(matches!(
            asked(&json!({ "state": "start", "audio": false })),
            Ok(Wanted::Start(Sound::Silent))
        ));
        let refusal = asked(&json!({ "state": "start", "audio": "loud" }))
            .err()
            .expect("a string is not true or false");
        assert_eq!(refusal.as_error()["data"]["code"], "E_INVALID_ARGS");
    }

    #[test]
    fn a_stop_says_which_recording_it_ends() {
        assert!(matches!(
            asked(&json!({ "state": "start" })),
            Ok(Wanted::Start(Sound::Silent))
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

    #[test]
    fn two_windows_record_and_stop_independently() {
        let mut recordings = Recordings::new();
        let one = recordings.start_silent(window("Calculator", 0x30)).unwrap();
        let two = recordings
            .start_silent(window("a.txt — Notepad", 0x31))
            .unwrap();
        assert_ne!(landed(&one), landed(&two));
        assert!(recordings.stop(id_of(&one)).is_ok());
        assert!(recordings.stop(id_of(&one)).is_err());
        assert!(recordings.stop(id_of(&two)).is_ok());
        for gone in [landed(&one), landed(&two)] {
            let _tidied = std::fs::remove_dir_all(gone);
        }
    }

    #[test]
    fn dropping_the_table_stops_every_recording_it_held() {
        let landing = {
            let mut recordings = Recordings::new();
            let started = recordings
                .start_silent(window("a window only this test names", 0x40))
                .unwrap();
            assert_eq!(recordings.running.len(), 1);
            landed(&started)
        };
        assert!(landing.is_dir(), "{} was not laid out", landing.display());
        let _tidied = std::fs::remove_dir_all(&landing);
    }
}
