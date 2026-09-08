// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which room a dispatch works in: the session a person named, or the
//! name the digest model gives the work when nobody did.

use kernel::{Address, AxCode, AxError};

use super::super::RunWorker;
use super::{NAME_THE_WORK, NAME_TOKENS};
use crate::assembly::credentials::dialect_headers;

impl RunWorker {
    /// Where a dispatch works: the room a named session opens, or the
    /// address it was sent to.
    ///
    /// A named session opens a room of its own under the building, so
    /// two sessions started from the same screen do not write over each
    /// other's files. An unnamed one is a person continuing what is
    /// already at that address.
    /// What this piece of work should be called, when the person did not
    /// say.
    ///
    /// A person who writes one sentence has named the work in it, and
    /// making them name it twice is the ceremony this interface exists
    /// to remove. So an address that names a building with no session
    /// beside it is answered here, by asking the cheapest model this
    /// city has for a short name.
    ///
    /// **A refusal, never a guess.** When no model answers, when the
    /// answer is not a legal session name, or when the address already
    /// names a room, this returns what it was given. The failure a
    /// person then sees names the field they have to fill, and the
    /// composer opens that one control — which is the whole reason the
    /// fallback is a refusal rather than a name this city made up. A run
    /// living in a room somebody did not choose cannot be found again by
    /// the name they would look for.
    pub(super) fn session_for(
        &mut self,
        addr: &Address,
        session: Option<kernel::SessionName>,
        task: &str,
    ) -> Result<Option<kernel::SessionName>, AxError> {
        if session.is_some() {
            return Ok(session);
        }
        // An address with a room in it is already a session: this is the
        // shape a second dispatch into an open session takes, and naming
        // it again would open a room inside a room.
        if addr.as_str().contains('/') {
            return Ok(None);
        }
        let Some(named) = self.name_the_work(task) else {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "work out what to call this work",
                addr.as_str().to_owned(),
            )
            .with_recovery(
                "name the room yourself: send the work to `building/name` rather than to \
                 `building`",
            ));
        };
        Ok(Some(named))
    }

    /// One cheap model call, turning a task into a short room name.
    ///
    /// The digest tag, which is this city's word for the small model
    /// that reads so the main one does not have to. Naming a piece of
    /// work is exactly that shape of job, and putting it on the main
    /// model would charge a person reasoning tokens for a filename.
    ///
    /// `None` for every failure this can have — no model registered for
    /// the tag, a call that did not come back, an answer that is not a
    /// legal session name. The caller turns that into a refusal naming
    /// the field, so a failure here costs a person one field rather than
    /// putting their work somewhere they will not look for it.
    fn name_the_work(&mut self, task: &str) -> Option<kernel::SessionName> {
        let chosen = self
            .book
            .select(kernel::ModelTag::Digest, &kernel::BuildingPolicy::default())
            .ok()?;
        let model_id = chosen.entry.id.clone();
        let mut adapter = gateway::adapter_for(
            &chosen,
            self.redemption(),
            dialect_headers(chosen.endpoint.dialect),
        )
        .ok()?;
        let answer = adapter
            .call(&kernel::ModelRequest {
                policy: kernel::BuildingPolicy::default(),
                segments: [kernel::B3Hash::digest(b""); 4],
                chat: kernel::ChatRequest {
                    model: model_id,
                    max_tokens: NAME_TOKENS,
                    system: vec![kernel::SystemBlock {
                        text: NAME_THE_WORK.to_owned(),
                        cache: false,
                    }],
                    messages: vec![kernel::ChatMessage {
                        role: kernel::Role::User,
                        content: vec![kernel::ContentBlock::Text {
                            text: task.to_owned(),
                        }],
                    }],
                    tools: Vec::new(),
                    effort: None,
                },
            })
            .ok()?;
        // The model is asked for one word and sometimes writes a
        // sentence around it. The first line, stripped of the
        // punctuation an answer tends to arrive wrapped in, is what is
        // offered to the parser — and the parser decides, not this.
        let said = kernel::content_from_message(&answer.message)
            .ok()?
            .into_iter()
            .find_map(|block| match block {
                kernel::ContentBlock::Text { text } => Some(text),
                _ => None,
            })?;
        let candidate = said
            .lines()
            .next()?
            .trim()
            .trim_matches(|glyph: char| glyph == '`' || glyph == '"' || glyph == '.');
        kernel::SessionName::parse(candidate).ok()
    }

    pub(super) fn room_for(
        &self,
        addr: Address,
        session: Option<&kernel::SessionName>,
    ) -> Result<Address, AxError> {
        match session {
            None => Ok(addr),
            Some(name) => {
                let building = city::Building::of(&addr)?;
                city::open_room(&self.city_root, building.addr(), name)
            }
        }
    }
}
