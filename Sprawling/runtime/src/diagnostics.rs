// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::redact::Marker;
use kernel::{RunId, Seq};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Refuse,

    Effect,

    Decide,

    Trace,

    Wire,
}

impl Level {
    pub const DEFAULT: Level = Level::Effect;

    pub const ALL: [Level; 5] = [
        Level::Refuse,
        Level::Effect,
        Level::Decide,
        Level::Trace,
        Level::Wire,
    ];

    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Level::Refuse => "refuse",
            Level::Effect => "effect",
            Level::Decide => "decide",
            Level::Trace => "trace",
            Level::Wire => "wire",
        }
    }

    #[must_use]
    pub fn parse(raw: &str) -> Option<Level> {
        Level::ALL.into_iter().find(|level| level.as_str() == raw)
    }
}

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Site<'a> {
    pub run: RunId,

    pub seq: Seq,
    pub module: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct Entry<'a> {
    pub level: Level,
    pub site: Site<'a>,
    pub message: &'a str,
}

pub type Sink = Box<dyn FnMut(Entry<'_>) + Send>;

pub struct Diagnostics {
    floor: Option<Level>,
    sink: Sink,
}

impl std::fmt::Debug for Diagnostics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Diagnostics")
            .field("floor", &self.floor)
            .finish_non_exhaustive()
    }
}

impl Diagnostics {
    #[must_use]
    pub fn new(floor: Level, sink: Sink) -> Diagnostics {
        Diagnostics {
            floor: Some(floor),
            sink,
        }
    }

    #[must_use]
    pub fn off() -> Diagnostics {
        Diagnostics {
            floor: None,
            sink: Box::new(|_entry: Entry<'_>| {}),
        }
    }

    #[must_use]
    pub fn floor(&self) -> Option<Level> {
        self.floor
    }

    #[must_use]
    pub fn admits(&self, level: Level) -> bool {
        self.floor.is_some_and(|floor| level <= floor)
    }

    pub fn write(&mut self, level: Level, site: Site<'_>, message: &str) {
        if !self.admits(level) {
            return;
        }
        let (scanned, _hits) = crate::redact::redact_text(message, Marker::Plain);
        (self.sink)(Entry {
            level,
            site,
            message: &scanned,
        });
    }
}

#[must_use]
pub fn render(entry: Entry<'_>) -> String {
    let Entry {
        level,
        site,
        message,
    } = entry;
    let mut map = serde_json::Map::new();
    map.insert(
        "level".to_owned(),
        serde_json::Value::String(level.as_str().to_owned()),
    );
    map.insert(
        "run".to_owned(),
        serde_json::Value::String(site.run.to_string()),
    );
    map.insert(
        "seq".to_owned(),
        serde_json::Value::Number(site.seq.value().into()),
    );
    map.insert(
        "module".to_owned(),
        serde_json::Value::String(site.module.to_owned()),
    );
    map.insert(
        "message".to_owned(),
        serde_json::Value::String(message.to_owned()),
    );
    serde_json::Value::Object(map).to_string()
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
    use std::sync::{Arc, Mutex};

    fn recorder() -> (Diagnostics, Arc<Mutex<Vec<String>>>) {
        at_floor(Level::DEFAULT)
    }

    fn at_floor(floor: Level) -> (Diagnostics, Arc<Mutex<Vec<String>>>) {
        let held = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&held);
        let log = Diagnostics::new(
            floor,
            Box::new(move |entry: Entry<'_>| {
                sink.lock().unwrap().push(render(entry));
            }),
        );
        (log, held)
    }

    fn site() -> Site<'static> {
        Site {
            run: RunId::CITY,
            seq: Seq::FIRST,
            module: "runtime::turn",
        }
    }

    #[test]
    fn the_default_floor_carries_the_two_levels_a_person_reads() {
        let (mut log, held) = recorder();
        for level in Level::ALL {
            log.write(level, site(), "something happened");
        }
        let lines = held.lock().unwrap();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("\"level\":\"refuse\""));
        assert!(lines[1].contains("\"level\":\"effect\""));
    }

    #[test]
    fn every_line_can_be_placed_in_the_history() {
        let (mut log, held) = recorder();
        log.write(Level::Effect, site(), "wrote notes.md");
        let lines = held.lock().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
        for field in ["run", "seq", "module"] {
            assert!(!parsed[field].is_null(), "{field} is required");
        }
        assert_eq!(parsed["seq"], Seq::FIRST.value());
        assert_eq!(parsed["module"], "runtime::turn");

        assert!(parsed.get("t").is_none());
        assert!(parsed.get("timestamp").is_none());
    }

    #[test]
    fn a_credential_that_reaches_a_line_does_not_leave_in_it() {
        let token = ["sk-live-", "Zk29fQ4t", "Rr7mVx1L", "pA6c"].concat();
        let (mut log, held) = recorder();
        log.write(
            Level::Effect,
            site(),
            &format!("calling the provider with {token}"),
        );
        let lines = held.lock().unwrap();
        assert!(!lines[0].contains(&token), "line: {}", lines[0]);
        assert!(lines[0].contains(&Marker::Plain.spell(token.as_bytes())));

        assert!(lines[0].contains("calling the provider with"));
    }

    #[test]
    fn a_message_with_nothing_to_hide_is_unchanged() {
        let (mut log, held) = recorder();
        log.write(Level::Effect, site(), "wrote notes.md");
        log.write(Level::Effect, site(), "");
        let lines = held.lock().unwrap();
        assert!(lines[0].contains("wrote notes.md"));
        assert!(lines[1].contains("\"message\":\"\""));
    }

    #[test]
    fn logging_off_writes_nothing_at_any_level() {
        let mut log = Diagnostics::off();
        assert_eq!(log.floor(), None);
        for level in Level::ALL {
            assert!(!log.admits(level));

            log.write(level, site(), "ignored");
        }
    }

    #[test]
    fn a_floor_admits_everything_at_or_before_it() {
        let (mut log, held) = at_floor(Level::Wire);
        for level in Level::ALL {
            log.write(level, site(), "x");
        }
        assert_eq!(held.lock().unwrap().len(), Level::ALL.len());
        assert!(Level::Refuse < Level::Effect);
        assert!(Level::Effect < Level::Wire);
    }

    #[test]
    fn a_level_is_named_the_same_way_on_the_command_line_and_in_a_line() {
        for level in Level::ALL {
            assert_eq!(Level::parse(level.as_str()), Some(level));
        }
        assert_eq!(Level::parse("verbose"), None);
        assert_eq!(Level::parse("off"), None);
    }
}
