// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Speaking into a run that is already working.
//!
//! Two entrances, one landing. The User's steer arrives as a control
//! surface command and never travels through the Inbox; an agent's steer
//! is a signal that overtakes the queue. Both land in the same place —
//! appended to the end of the next tool result — because the model
//! should have to recognise one shape, not two.
//!
//! The entrances stay apart for a different reason than the landing
//! stays together: content that claims to come from the User must not
//! be able to render as the User. The two are two types: a [`Steer`] is
//! the User's text and nothing else, and a [`Letter`] carries the
//! resident who sent it, which `runtime::conversation` renders inside an
//! envelope its body cannot close (collab D16).

use kernel::{Address, AxCode, AxError, RunId, TimeMs, Version};

use crate::inbox::{SenderState, Signal};
use kernel::event::record::{SignalId, SignalKind};

/// The User's steer at its landing: what the User said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Steer {
    text: String,
}

impl Steer {
    /// The control surface's entrance.
    ///
    /// # Errors
    /// Refuses empty text: an interruption that says nothing costs a
    /// turn and gives the run nothing to act on.
    pub fn from_person(text: &str) -> Result<Steer, AxError> {
        Ok(Steer {
            text: non_empty(text)?,
        })
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// Why a resident's words reached a working run's window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LetterKind {
    /// A steer-kind signal, taken at a safe point.
    Steer,
    /// The reply a sync wait ended with (collab D9).
    Reply,
}

/// A resident's words at their landing, with who sent them as the city
/// stamped it rather than as the body claims (collab D16).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Letter {
    from: String,
    run: Option<RunId>,
    kind: LetterKind,
    sender: Option<SenderState>,
    text: String,
}

impl Letter {
    /// The Inbox's entrance: a steer-kind signal, attributed to the
    /// resident that sent it.
    ///
    /// # Errors
    /// Refuses a signal of any other kind — an ordinary mention does not
    /// get to interrupt — and one whose payload carries no text.
    pub fn from_signal(signal: &Signal) -> Result<Letter, AxError> {
        if signal.kind() != SignalKind::Steer {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "read a signal as a steer",
                signal.kind().as_str().to_owned(),
            )
            .with_recovery("only a steer-kind signal overtakes; deliver the rest to the inbox"));
        }
        let letter = Letter::reply(signal);
        Ok(Letter {
            text: non_empty(&letter.text)?,
            kind: LetterKind::Steer,
            ..letter
        })
    }

    /// The reply a sync wait ended with, whatever kind of signal carried
    /// it. An empty reply still ends the wait, so it is not refused.
    pub(crate) fn reply(signal: &Signal) -> Letter {
        Letter {
            from: signal.from().to_owned(),
            run: signal.run(),
            kind: LetterKind::Reply,
            sender: signal.sender(),
            text: signal
                .payload()
                .as_map()
                .get("text")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_owned(),
        }
    }

    /// The sending room's address, without the `@`.
    #[must_use]
    pub fn from(&self) -> &str {
        &self.from
    }

    /// The run that wrote the `signal_enqueued` line, where the city
    /// stamped it.
    #[must_use]
    pub fn run(&self) -> Option<RunId> {
        self.run
    }

    #[must_use]
    pub fn kind(&self) -> LetterKind {
        self.kind
    }

    /// Where the sender stood when the city delivered it (collab D10).
    #[must_use]
    pub fn sender(&self) -> Option<SenderState> {
        self.sender
    }

    /// The body as the sender wrote it, trimmed; the renderer escapes it.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// A steer on its way out of one resident and into another's inbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSteer {
    id: String,
    text: String,
}

impl AgentSteer {
    /// # Errors
    /// Refuses an unnamed sender and empty text.
    pub fn new(id: &str, text: &str) -> Result<AgentSteer, AxError> {
        if id.is_empty() {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "build an agent steer",
                "unnamed sender".to_owned(),
            )
            .with_recovery("a steer is read as coming from someone; give the sender's address"));
        }
        Ok(AgentSteer {
            id: id.to_owned(),
            text: non_empty(text)?,
        })
    }

    /// The signal this steer travels as. It carries its text in the
    /// payload under `text`, which is where [`Letter::from_signal`] reads
    /// it: one writer, one reader.
    ///
    /// # Errors
    /// Propagates the signal's own refusals.
    pub fn signal(
        &self,
        id: SignalId,
        room: Address,
        room_version: Version,
        at: TimeMs,
    ) -> Result<Signal, AxError> {
        let mut body = serde_json::Map::new();
        body.insert(
            "text".to_owned(),
            serde_json::Value::String(self.text.clone()),
        );
        Signal::new(
            id,
            SignalKind::Steer,
            self.id.clone(),
            room,
            room_version,
            kernel::Payload::new(body)?,
            at,
        )
    }

    /// What this steer looks like where it lands, before the city
    /// stamps its delivery.
    #[must_use]
    pub fn landing(&self) -> Letter {
        Letter {
            from: self.id.clone(),
            run: None,
            kind: LetterKind::Steer,
            sender: None,
            text: self.text.clone(),
        }
    }
}

/// What a `pull` row says about where the sender stood when the signal
/// was delivered (collab D10).
pub(crate) fn sender_note(state: SenderState) -> String {
    format!(
        "(the sender's run was {} when this arrived)",
        state.as_str()
    )
}

fn non_empty(text: &str) -> Result<String, AxError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(AxError::failure(
            AxCode::InvalidArgs,
            "build a steer",
            "empty text".to_owned(),
        )
        .with_recovery("say what should change; a steer with no words costs a turn"));
    }
    Ok(trimmed.to_owned())
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
    use kernel::Payload;

    fn room() -> Address {
        Address::parse("lab/room2").unwrap()
    }

    /// D10, D16: the landing carries where the sender stood when the
    /// city delivered the signal and the run that sent it, as stamps
    /// rather than words in the body.
    #[test]
    fn a_delivered_steer_lands_with_the_stamps_of_its_delivery() {
        let run = RunId::parse("0198f6a2-7c4a-7bbb-9d1e-000000000001").unwrap();
        let signal = AgentSteer::new("lab/room1", "stop the kiln")
            .unwrap()
            .signal(
                SignalId::parse("s-1").unwrap(),
                room(),
                Version::new(1),
                TimeMs::new(9),
            )
            .unwrap()
            .delivered(SenderState::Cancelled)
            .sent_by(run);
        let landed = Letter::from_signal(&signal).unwrap();
        assert_eq!(
            landed,
            Letter {
                from: "lab/room1".to_owned(),
                run: Some(run),
                kind: LetterKind::Steer,
                sender: Some(SenderState::Cancelled),
                text: "stop the kiln".to_owned(),
            }
        );
    }

    #[test]
    fn an_agent_steer_travels_as_a_signal_and_lands_as_itself() {
        let agent = AgentSteer::new("lab/room1", "check the units").unwrap();
        let signal = agent
            .signal(
                SignalId::parse("s-1").unwrap(),
                room(),
                Version::new(2),
                TimeMs::new(9),
            )
            .unwrap();
        assert_eq!(signal.kind(), SignalKind::Steer);

        let landed = Letter::from_signal(&signal).unwrap();
        assert_eq!(landed, agent.landing());
        assert_eq!(landed.text(), "check the units");
    }

    #[test]
    fn an_ordinary_mention_does_not_get_to_interrupt() {
        let mut body = serde_json::Map::new();
        body.insert(
            "text".to_owned(),
            serde_json::Value::String("have a look".to_owned()),
        );
        let mention = Signal::new(
            SignalId::parse("s-2").unwrap(),
            SignalKind::Mention,
            "lab/room1".to_owned(),
            room(),
            Version::new(1),
            Payload::new(body).unwrap(),
            TimeMs::new(9),
        )
        .unwrap();

        let err = Letter::from_signal(&mention).unwrap_err();
        assert_eq!(err.code(), &AxCode::InvalidArgs);
        assert!(err.recovery().contains("inbox"));
    }

    #[test]
    fn a_steer_with_nothing_in_it_is_refused_at_every_entrance() {
        assert!(Steer::from_person("   ").is_err());
        assert!(AgentSteer::new("lab/room1", "\n").is_err());
        assert!(AgentSteer::new("", "text").is_err());
    }

    #[test]
    fn the_text_arrives_trimmed_because_the_window_shows_it_verbatim() {
        assert_eq!(
            Steer::from_person("  wrap up \n").unwrap().text(),
            "wrap up"
        );
    }
}
