// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which room a dispatch works in: the session a person named, or the
//! name this city takes from the task by rule when nobody did
//! (sprawling-SPEC.md 8-86).

use kernel::{Address, AxError, SessionName};

use super::super::RunWorker;

/// How much of one word a rule name keeps. Four words of this length
/// and three hyphens are 63 characters, inside the session name limit,
/// so a rule name is always a legal one.
const RULE_WORD_MAX: usize = 15;

/// How many words of the task a rule name keeps.
const RULE_WORDS: usize = 4;

/// What a task with no ASCII word in it is called; `city::open_room`
/// suffixes it when the room is taken.
const UNWORDED: &str = "work";

impl RunWorker {
    pub(super) fn room_for(
        &self,
        addr: Address,
        session: Option<&SessionName>,
    ) -> Result<Address, AxError> {
        match session {
            // A room address nobody opened: the city makes it here, the
            // first thing it writes for the dispatch, and seals it, so
            // nothing the run writes in it reaches the project's git.
            None => {
                city::claim_room(&self.city_root, &addr)?;
                Ok(addr)
            }
            Some(name) => {
                let building = city::Building::of(&addr)?;
                city::open_room(&self.city_root, building.addr(), name)
            }
        }
    }
}

/// Where a dispatch works: the session a person named, the room the
/// address already names, or a room named from the task by rule.
///
/// A person who writes one sentence has named the work in it, and
/// making them name it twice is the ceremony this interface exists
/// to remove. The name is taken from the task's own words rather
/// than asked of a model, so it costs no call before the run's
/// first token and no reply can turn into a room a person cannot
/// find again.
///
/// # Errors
/// Propagates `rule_name`, which does not fail on any task.
pub(super) fn session_for(
    addr: &Address,
    session: Option<SessionName>,
    task: &str,
) -> Result<Option<SessionName>, AxError> {
    match session {
        None if opens_a_room(addr, None) => rule_name(task).map(Some),
        None | Some(_) => Ok(session),
    }
}

/// Whether a dispatch to `addr` opens a room of its own: it named a
/// session, or it was sent to a building. An address with a room in it
/// is already a session: this is the shape a second dispatch into an
/// open session takes, and naming it again would open a room inside a
/// room.
pub(super) fn opens_a_room(addr: &Address, session: Option<&SessionName>) -> bool {
    session.is_some() || !addr.as_str().contains('/')
}

/// The room name a task earns: its first four ASCII words, lowercased,
/// each cut to fifteen characters, joined by hyphens.
///
/// A task with no ASCII word, or whose words spell a name the city
/// keeps for itself, is called `work`; `city::open_room` suffixes it
/// when the room is taken, so the fallback never shares a room.
///
/// # Errors
/// Propagates `SessionName::parse`, which stays the authority on what
/// a session name is and accepts `work`.
pub(super) fn rule_name(task: &str) -> Result<SessionName, AxError> {
    let words = task
        .split(|glyph: char| !glyph.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .take(RULE_WORDS)
        .map(|word| {
            word.chars()
                .take(RULE_WORD_MAX)
                .map(|glyph| glyph.to_ascii_lowercase())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("-");
    match SessionName::parse(&words) {
        Ok(name) => Ok(name),
        Err(_unspellable) => SessionName::parse(UNWORDED),
    }
}
