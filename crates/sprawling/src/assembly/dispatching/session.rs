// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which room a dispatch works in: the session a person named, or the
//! name the digest model gives the work when nobody did.

use kernel::{Address, AxCode, AxError};

use super::super::RunWorker;
use super::{NAME_THE_WORK, NAME_TOKENS};

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
    ///
    /// `policy` is the building's own, read by the caller from the rules
    /// it already loaded: naming the work sends the task text to a
    /// model, so a confidential building governs this call exactly as it
    /// governs the run's own calls.
    ///
    /// # Errors
    /// Refuses when no legal name could be had, and propagates the
    /// book's refusal to route a confidential building's text to a model
    /// that is not on this machine.
    pub(super) fn session_for(
        &mut self,
        addr: &Address,
        session: Option<kernel::SessionName>,
        task: &str,
        policy: &kernel::BuildingPolicy,
    ) -> Result<Option<kernel::SessionName>, AxError> {
        // An address with a room in it is already a session: this is the
        // shape a second dispatch into an open session takes, and naming
        // it again would open a room inside a room.
        if !needs_a_name(addr, session.as_ref()) {
            return Ok(session);
        }
        self.naming_call(policy)?
            .name(task)?
            .map(Some)
            .ok_or_else(|| unnamed(addr))
    }

    /// The digest model's call, built here and made wherever the caller
    /// can afford to wait for it.
    ///
    /// The choice is made under the building's policy and refused here
    /// rather than swallowed: a confidential building that has no digest
    /// model on this machine is a decision for the person, and the two
    /// ways out are both named.
    ///
    /// # Errors
    /// Propagates the book's refusal to route a confidential building's
    /// text off this machine, the vault's refusal to redeem, and the
    /// adapter's refusal to be built.
    pub(super) fn naming_call(
        &self,
        policy: &kernel::BuildingPolicy,
    ) -> Result<NamingCall, AxError> {
        let chosen = self
            .credentials
            .book
            .select(kernel::ModelTag::Digest, policy)
            .map_err(|refused| {
                refused.rewrite_recovery(
                    "name the room yourself by sending the work to `building/name`, or choose a \
                     digest model that runs on this machine",
                )
            })?;
        let adapter = self.models.build(&chosen, self.redemption()?)?;
        Ok(NamingCall {
            adapter,
            model: chosen.entry.id.clone(),
            policy: policy.clone(),
        })
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

/// Whether a dispatch waits on the digest model before it has a room.
///
/// An address with a room in it is already a session, so only a bare
/// building with no session beside it is named.
pub(in crate::assembly) fn needs_a_name(
    addr: &Address,
    session: Option<&kernel::SessionName>,
) -> bool {
    session.is_none() && !addr.as_str().contains('/')
}

/// The refusal owed when the digest model answered with something that
/// is not a legal session name. It names the field the person has to
/// fill, and the composer opens that one control.
pub(super) fn unnamed(addr: &Address) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "work out what to call this work",
        addr.as_str().to_owned(),
    )
    .with_recovery(
        "name the room yourself: send the work to `building/name` rather than to `building`",
    )
}

/// One cheap model call, turning a task into a short room name.
///
/// The digest tag, which is this city's word for the small model that
/// reads so the main one does not have to. Naming a piece of work is
/// exactly that shape of job, and putting it on the main model would
/// charge a person reasoning tokens for a filename.
///
/// Built on the accounting thread, because choosing the model reads the
/// book and the vault; made anywhere, because it owns everything it
/// needs and the call is the part that waits on a provider. The policy
/// it carries is the building's own, so the task text travels under the
/// rules of the building it was sent to.
pub(super) struct NamingCall {
    adapter: Box<dyn kernel::Model + Send>,
    model: String,
    policy: kernel::BuildingPolicy,
}

impl NamingCall {
    /// Asks the model, and reads a session name out of what it says.
    ///
    /// `Ok(None)` is reserved for one outcome: **the model answered, and
    /// what it answered is not a legal session name.** A call that did
    /// not come back and a reply whose content cannot be read are each
    /// returned as the error they are, because a person told only "name
    /// the room yourself" would never learn that their provider was
    /// unreachable (sprawling-SPEC.md 8-75).
    ///
    /// # Errors
    /// Propagates the provider's own failure and a reply this build
    /// cannot read.
    pub(super) fn name(mut self, task: &str) -> Result<Option<kernel::SessionName>, AxError> {
        let answer = self.adapter.call(&kernel::ModelRequest {
            policy: self.policy,
            segments: [kernel::B3Hash::digest(b""); 4],
            chat: kernel::ChatRequest {
                model: self.model,
                max_tokens: NAME_TOKENS,
                system: vec![kernel::SystemBlock {
                    text: NAME_THE_WORK.to_owned(),
                    cache: false,
                }],
                messages: vec![kernel::ChatMessage {
                    cache: false,
                    role: kernel::Role::User,
                    content: vec![kernel::ContentBlock::Text {
                        text: task.to_owned(),
                    }],
                }],
                tools: Vec::new(),
                effort: None,
            },
        })?;
        // The model is asked for one word and sometimes writes a
        // sentence around it. The first line, stripped of the
        // punctuation an answer tends to arrive wrapped in, is what is
        // offered to the parser — and the parser decides, not this.
        let spoken = kernel::model::content_from_message(&answer.message)?
            .into_iter()
            .find_map(|block| match block {
                kernel::ContentBlock::Text { text } => Some(text),
                kernel::ContentBlock::Thinking { .. }
                | kernel::ContentBlock::RedactedThinking { .. }
                | kernel::ContentBlock::ToolUse { .. }
                | kernel::ContentBlock::Image(_)
                | kernel::ContentBlock::ToolResult { .. } => None,
            });
        // A reply with no words in it is a model that did not name the
        // work, which is the one outcome the caller turns into a field
        // for the person to fill.
        let Some(said) = spoken else {
            return Ok(None);
        };
        let Some(first) = said.lines().next() else {
            return Ok(None);
        };
        let candidate = first
            .trim()
            .trim_matches(|glyph: char| glyph == '`' || glyph == '"' || glyph == '.');
        Ok(kernel::SessionName::parse(candidate).ok())
    }
}
