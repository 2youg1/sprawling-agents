// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The live region as the renderer reads it (`crates/sprawling/spec/Console.lean`
//! §8-11): what waits for the person, who is working, the calls under
//! way, the question `/quit` asks, and the composer with its menu, all
//! borrowed from the writer's state for one frame.

use console_ffi::scene::{Calling, Composer, Entry, Live, Waiting, Working};

use super::super::cli::Session;
use super::super::local_time;
use super::super::stream::resident;
use super::CALLS_SHOWN;
use super::menu::Menu;

/// The composer's half of the live region.
pub(super) struct Composing<'a> {
    pub(super) typed: &'a str,
    pub(super) cursor: usize,
    pub(super) placeholder: &'a str,
    pub(super) offer: &'a str,
}

/// The live region: the oldest request waiting, the run working, the
/// newest calls under way, the question `/quit` asks, and the composer
/// with its menu.
pub(super) fn live<'a>(
    session: Option<&'a Session>,
    composing: Composing<'a>,
    menu: Option<&'a Menu>,
    asking: Option<u64>,
) -> Live<'a> {
    let seen = session.map(|session| &session.seen);
    let waiting = seen.and_then(|seen| {
        seen.waiting.first().map(|first| Waiting {
            what: &first.what,
            more: seen.waiting.len().saturating_sub(1),
        })
    });
    let working = session.and_then(|session| {
        session.seen.run.map(|_| Working {
            resident: resident(&session.room),
            since: session.seen.since().and_then(local_time::local),
        })
    });
    let calling = seen.map_or_else(Vec::new, |seen| {
        let skip = seen.calls.len().saturating_sub(CALLS_SHOWN);
        seen.calls
            .iter()
            .skip(skip)
            .map(|call| Calling {
                name: &call.name,
                subject: &call.subject,
            })
            .collect()
    });
    let menu = menu.map(|menu| {
        let (shown, chosen, more) = menu.window();
        console_ffi::scene::Menu {
            shown,
            chosen,
            more,
        }
    });
    Live {
        waiting,
        working,
        calling,
        asking,
        composer: Composer {
            typed: composing.typed,
            cursor: composing.cursor,
            placeholder: composing.placeholder,
            room: session.map_or(kernel::consts_policy::HALL_MAYOR, |session| {
                session.room.as_str()
            }),
            offer: composing.offer,
        },
        menu,
    }
}

/// What the empty composer says: who plain lines go to.
pub(super) fn placeholder(session: Option<&Session>) -> String {
    match session {
        Some(session) if session.room.as_str() == kernel::consts_policy::HALL_MAYOR => {
            "to the Mayor…".to_owned()
        }
        Some(session) => format!("to {}…", resident(&session.room)),
        None => String::new(),
    }
}

/// A line the console says, as a transcript note: the two spaces every
/// console line starts with are the renderer's to place.
pub(super) fn note(line: &str) -> Entry {
    let said: Vec<&str> = line
        .trim_start_matches('\n')
        .lines()
        .map(|line| line.strip_prefix("  ").unwrap_or(line))
        .collect();
    Entry::Note {
        said: said.join("\n"),
    }
}
