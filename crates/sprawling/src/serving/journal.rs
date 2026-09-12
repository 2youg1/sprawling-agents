// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one sink every diagnostic line leaves this process through, and
//! the two mouths it leaves by: standard error, and whoever has the log
//! lens open.
//!
//! **This is not a second log.** A page reads the same entry the
//! terminal reads, at the same floor, from the same sink, and the wire
//! frame it becomes is discardable in exactly the way `docs/logging.md`
//! says a log is: no ledger sequence of its own, never written down,
//! and a client that missed one has lost nothing. Nothing here reads a
//! line back, so the rule that decision logic reads no log output is
//! unchanged.
//!
//! **The clock is sampled here** because this is the assembly layer,
//! which is where sampling is sanctioned (`docs/logging.md` section 8);
//! the library that writes the entry is not allowed a second time
//! source. A clock that cannot be read leaves the line's `t` absent
//! rather than dropping the line, because the ledger position is what
//! anchors it and the diagnostic is what the reader came for.

use runtime::diagnostics::{Entry, Sink};

/// How many lines a page that stopped reading may fall behind before it
/// starts losing them.
///
/// Larger than the increment channel and smaller than the event one: a
/// city at the `wire` floor writes faster than a person reads, and what
/// is lost here is a diagnostic rather than history.
const DEPTH: usize = 512;

/// Where the process log goes on its way out of this process.
pub struct Journal {
    lines: tokio::sync::broadcast::Sender<channels::LogLine>,
}

impl Journal {
    #[must_use]
    pub fn new() -> Journal {
        let (lines, _watching) = tokio::sync::broadcast::channel(DEPTH);
        Journal { lines }
    }

    /// The sink a `Diagnostics` is built with: every admitted line to
    /// standard error, and the same line to whoever is watching.
    ///
    /// A city with no page open still writes to the terminal, because
    /// the send failing means nobody subscribed and that is what a city
    /// nobody is looking at looks like.
    #[must_use]
    pub fn sink(&self) -> Sink {
        let lines = self.lines.clone();
        Box::new(move |entry: Entry<'_>| {
            eprintln!("{}", runtime::diagnostics::render(entry));
            let _ = lines.send(carried(entry));
        })
    }

    /// The broadcast the socket subscribes to.
    pub(super) fn lines(&self) -> tokio::sync::broadcast::Sender<channels::LogLine> {
        self.lines.clone()
    }
}

impl Default for Journal {
    fn default() -> Journal {
        Journal::new()
    }
}

/// One entry in the shape the wire carries.
///
/// The nil run is the city speaking for itself, and it travels as an
/// absent run: a page filtering by run would otherwise offer a
/// session-shaped identifier that names no session.
fn carried(entry: Entry<'_>) -> channels::LogLine {
    channels::LogLine {
        seq: entry.site.seq,
        t: crate::assembly::now_ms().ok(),
        level: level(entry.level),
        module: entry.site.module.to_owned(),
        run: (entry.site.run != kernel::RunId::CITY).then_some(entry.site.run),
        line: entry.message.to_owned(),
    }
}

/// The one mapping between the level a terminal spells and the level a
/// page spells. Exhaustive, so a sixth level cannot be added upstream
/// without this file deciding what a page calls it.
fn level(level: runtime::diagnostics::Level) -> channels::LogLevel {
    match level {
        runtime::diagnostics::Level::Refuse => channels::LogLevel::Refuse,
        runtime::diagnostics::Level::Effect => channels::LogLevel::Effect,
        runtime::diagnostics::Level::Decide => channels::LogLevel::Decide,
        runtime::diagnostics::Level::Trace => channels::LogLevel::Trace,
        runtime::diagnostics::Level::Wire => channels::LogLevel::Wire,
        // `Level` is `#[non_exhaustive]`, so the compiler cannot close
        // this match for us. A level this build does not know is
        // reported as the widest one a person reads rather than
        // dropped: a line nobody can see is worse than a line filed
        // one row too high.
        _ => channels::LogLevel::Refuse,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;
    use runtime::diagnostics::{Level, Site};

    fn entry_at(run: kernel::RunId, level: Level) -> channels::LogLine {
        carried(Entry {
            level,
            site: Site {
                run,
                seq: kernel::Seq::new(9),
                module: "bin::assembly",
            },
            message: "wrote notes.md",
        })
    }

    /// The five names a page spells are the five names a terminal
    /// spells. Held here because `channels` cannot depend on `runtime`,
    /// so the two enums would otherwise be free to drift apart while
    /// both builds stayed green.
    #[test]
    fn a_level_is_named_the_same_way_on_a_terminal_and_on_a_page() {
        for level in Level::ALL {
            let carried = entry_at(kernel::RunId::CITY, level);
            let spelled = serde_json::to_string(&carried.level).unwrap();
            assert_eq!(spelled, format!("\"{}\"", level.as_str()));
        }
    }

    /// The whole path, from the call a module makes to the frame a
    /// page would receive: the floor is honoured, the fields arrive,
    /// and the scan that protects the ledger protects this too.
    #[test]
    fn a_written_line_reaches_a_watcher_with_its_credential_already_gone() {
        let journal = Journal::new();
        let mut watching = journal.lines().subscribe();
        let mut log = runtime::diagnostics::Diagnostics::new(
            runtime::diagnostics::Level::Effect,
            journal.sink(),
        );
        // Assembled at runtime, so no credential-shaped literal is at
        // rest in this repository.
        let token = ["sk-live-", "Zk29fQ4t", "Rr7mVx1L", "pA6c"].concat();
        let site = Site {
            run: kernel::RunId::CITY,
            seq: kernel::Seq::new(4),
            module: "bin::doctor",
        };
        log.write(Level::Effect, site, &format!("calling with {token}"));
        // Below the floor, so nothing of it travels.
        log.write(Level::Trace, site, "retrying");
        let carried = watching.try_recv().expect("the line reached the watcher");
        assert_eq!(carried.level, channels::LogLevel::Effect);
        assert_eq!(carried.module, "bin::doctor");
        assert_eq!(carried.seq, kernel::Seq::new(4));
        assert_eq!(carried.run, None);
        assert!(!carried.line.contains(&token), "{}", carried.line);
        assert!(carried.line.contains(runtime::diagnostics::REDACTED));
        assert!(
            watching.try_recv().is_err(),
            "a level the floor refuses reaches no page either"
        );
    }

    /// The city speaking for itself carries no run, and a resident's
    /// line carries the one it was written under.
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
