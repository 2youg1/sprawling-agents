// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, AxError, SessionName};

use super::super::RunWorker;

const RULE_WORD_MAX: usize = 15;

const RULE_WORDS: usize = 4;

const UNWORDED: &str = "work";

impl RunWorker {
    pub(super) fn room_for(
        &self,
        addr: Address,
        session: Option<&SessionName>,
    ) -> Result<Address, AxError> {
        match session {
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

pub(super) fn opens_a_room(addr: &Address, session: Option<&SessionName>) -> bool {
    session.is_some() || !addr.as_str().contains('/')
}

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
