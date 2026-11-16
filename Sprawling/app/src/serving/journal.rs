// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::{Mutex, PoisonError};

use kernel::{Seq, TimeMs};
use runtime::diagnostics::{Diagnostics, Entry, Level, Sink, Site};

pub type Clock = std::sync::Arc<dyn accounting::Clock + Send + Sync>;

const DEPTH: usize = 512;

static PROCESS: Mutex<Option<(Diagnostics, Seq)>> = Mutex::new(None);

pub(crate) fn notice(module: &'static str, message: &str) {
    let mut process = PROCESS.lock().unwrap_or_else(PoisonError::into_inner);
    match process.as_mut() {
        Some((log, seq)) => log.write(
            Level::Refuse,
            Site {
                run: kernel::RunId::CITY,
                seq: *seq,
                module,
            },
            message,
        ),
        None => eprintln!("{message}"),
    }
}

pub struct Journal {
    lines: tokio::sync::broadcast::Sender<wire::LogLine>,
    clock: Clock,
    mouth: Mouth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mouth {
    StandardError,
    LensOnly,
}

impl Journal {
    #[must_use]
    pub fn new(clock: Clock) -> Journal {
        Journal {
            lines: tokio::sync::broadcast::Sender::new(DEPTH),
            clock,
            mouth: Mouth::StandardError,
        }
    }

    #[must_use]
    pub fn lens_only(self) -> Journal {
        Journal {
            mouth: Mouth::LensOnly,
            ..self
        }
    }

    #[must_use]
    pub fn beside(
        &self,
        log: &runtime::diagnostics::Diagnostics,
    ) -> runtime::diagnostics::Diagnostics {
        log.floor()
            .map_or_else(runtime::diagnostics::Diagnostics::off, |floor| {
                runtime::diagnostics::Diagnostics::new(floor, self.sink())
            })
    }

    pub fn sink(&self) -> Sink {
        let lines = self.lines.clone();
        let clock = std::sync::Arc::clone(&self.clock);
        let mouth = self.mouth;
        Box::new(move |entry: Entry<'_>| {
            if mouth == Mouth::StandardError {
                eprintln!("{}", runtime::diagnostics::render(entry));
            }
            drop(lines.send(carried(entry, clock.now().ok())));
        })
    }

    pub(crate) fn hears_the_process(&self, log: &Diagnostics, at: Seq) {
        let writer = self.beside(log);
        *PROCESS.lock().unwrap_or_else(PoisonError::into_inner) = Some((writer, at));
    }

    pub(crate) fn lines(&self) -> tokio::sync::broadcast::Sender<wire::LogLine> {
        self.lines.clone()
    }
}

fn carried(entry: Entry<'_>, t: Option<TimeMs>) -> wire::LogLine {
    wire::LogLine {
        seq: entry.site.seq,
        t,
        level: level(entry.level),
        module: entry.site.module.to_owned(),
        run: (entry.site.run != kernel::RunId::CITY).then_some(entry.site.run),
        line: entry.message.to_owned(),
    }
}

fn level(level: runtime::diagnostics::Level) -> wire::LogLevel {
    match level {
        runtime::diagnostics::Level::Refuse => wire::LogLevel::Refuse,
        runtime::diagnostics::Level::Effect => wire::LogLevel::Effect,
        runtime::diagnostics::Level::Decide => wire::LogLevel::Decide,
        runtime::diagnostics::Level::Trace => wire::LogLevel::Trace,
        runtime::diagnostics::Level::Wire => wire::LogLevel::Wire,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;
    use runtime::diagnostics::{Level, Site};

    fn entry_at(run: kernel::RunId, level: Level) -> wire::LogLine {
        carried(
            Entry {
                level,
                site: Site {
                    run,
                    seq: kernel::Seq::new(9),
                    module: "bin::assembly",
                },
                message: "wrote notes.md",
            },
            None,
        )
    }

    #[test]
    fn a_level_is_named_the_same_way_on_a_terminal_and_on_a_page() {
        for level in Level::ALL {
            let carried = entry_at(kernel::RunId::CITY, level);
            let spelled = serde_json::to_string(&carried.level).unwrap();
            assert_eq!(spelled, format!("\"{}\"", level.as_str()));
        }
    }

    #[test]
    fn a_written_line_reaches_a_watcher_with_its_credential_already_gone() {
        let journal = Journal::new(std::sync::Arc::new(crate::assembly::SystemClock));
        let mut watching = journal.lines().subscribe();
        let mut log = runtime::diagnostics::Diagnostics::new(
            runtime::diagnostics::Level::Effect,
            journal.sink(),
        );

        let token = ["sk-live-", "Zk29fQ4t", "Rr7mVx1L", "pA6c"].concat();
        let site = Site {
            run: kernel::RunId::CITY,
            seq: kernel::Seq::new(4),
            module: "bin::doctor",
        };
        log.write(Level::Effect, site, &format!("calling with {token}"));

        log.write(Level::Trace, site, "retrying");
        let carried = watching.try_recv().expect("the line reached the watcher");
        assert_eq!(carried.level, wire::LogLevel::Effect);
        assert_eq!(carried.module, "bin::doctor");
        assert_eq!(carried.seq, kernel::Seq::new(4));
        assert_eq!(carried.run, None);
        assert!(!carried.line.contains(&token), "{}", carried.line);
        assert!(
            carried
                .line
                .contains(&runtime::redact::Marker::Plain.spell(token.as_bytes()))
        );
        assert!(
            watching.try_recv().is_err(),
            "a level the floor refuses reaches no page either"
        );
    }

    #[test]
    fn the_city_speaks_without_a_run_and_a_resident_speaks_with_one() {
        assert_eq!(entry_at(kernel::RunId::CITY, Level::Effect).run, None);
        let resident = kernel::RunId::from_bytes([3u8; 16]);
        let line = entry_at(resident, Level::Effect);
        assert_eq!(line.run, Some(resident));
        assert_eq!(line.seq, kernel::Seq::new(9));
        assert_eq!(line.module, "bin::assembly");
        assert_eq!(line.line, "wrote notes.md");
    }
}
