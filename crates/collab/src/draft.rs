// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What happens when the room moved while you were writing.
//!
//! A draft carries the room version its author saw. If the room has
//! moved by the time it arrives, the draft is held and the author is
//! shown four ways back: rewrite, send as is, withdraw, or send anyway
//! having been told. All four are offered rather than rewriting by
//! default, because "the room moved" is not the same as "this is now
//! wrong", and only the author can tell which.
//!
//! Sending anyway is gated by a token this module issues, bound to the
//! room version the author was actually shown, and void the moment the
//! room moves again. The general rule it comes from: a bypass on a
//! coordination gate must confirm state the server has shown, never be
//! an option the client can simply pass. An unconditional bypass
//! parameter gets learned as a precaution by any model asked to be
//! efficient - it saves a round trip, and the model does not know what
//! it walked past - so the gate stops existing without anyone deciding
//! to remove it.

use std::collections::BTreeMap;

use kernel::{Address, AxError, Payload, Version, consts_policy::DRAFT_HELD_ESCALATE};
use serde_json::{Map, Value};

/// A message on its way into a room.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    author: String,
    room: Address,
    seen: Version,
    body: Payload,
}

impl Draft {
    #[must_use]
    pub fn new(author: String, room: Address, seen: Version, body: Payload) -> Draft {
        Draft {
            author,
            room,
            seen,
            body,
        }
    }

    #[must_use]
    pub fn author(&self) -> &str {
        &self.author
    }

    #[must_use]
    pub fn room(&self) -> &Address {
        &self.room
    }

    /// The room version the author was working against.
    #[must_use]
    pub fn seen(&self) -> Version {
        self.seen
    }

    #[must_use]
    pub fn body(&self) -> &Payload {
        &self.body
    }
}

/// The four ways back from a hold. Exhaustive: a fifth would be a new
/// answer to "the room moved", and every caller has to say what it does
/// about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Return {
    /// Rewrite against the change summary and submit again.
    Rewrite,
    /// Submit the same words again; it may be held again.
    SendAsIs,
    /// Drop it.
    Withdraw,
    /// Send it having been shown the change. Consumes a hold token.
    ForceInformed,
}

impl Return {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Return::Rewrite => "rewrite",
            Return::SendAsIs => "send_as_is",
            Return::Withdraw => "withdraw",
            Return::ForceInformed => "force_informed",
        }
    }
}

/// Proof that this author was shown this room at this version. Issued
/// here, never constructed by a caller: the fields are private and there
/// is no public constructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HoldToken {
    shown: Version,
    turn: u32,
}

impl HoldToken {
    /// The room version the author was shown when this was issued.
    #[must_use]
    pub fn shown(&self) -> Version {
        self.shown
    }

    /// The turn it was issued in. It does not outlive that turn, and the
    /// turn number is a parameter rather than a clock reading.
    #[must_use]
    pub fn turn(&self) -> u32 {
        self.turn
    }
}

/// What became of a draft that arrived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Submission {
    Delivered,
    Held {
        token: HoldToken,
        holds: u32,
    },
    /// Held so many times in a row that a person decides now.
    Escalated {
        holds: u32,
    },
}

/// What became of an author's answer to a hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    Delivered,
    Withdrawn,
    Held {
        token: HoldToken,
        holds: u32,
    },
    Escalated {
        holds: u32,
    },
    /// The bypass was refused because the room moved after the hold was
    /// shown; a fresh hold is registered with the version now current.
    TokenVoid {
        token: HoldToken,
    },
}

/// The desk that holds drafts and issues tokens.
#[derive(Debug, Default)]
pub struct Drafts {
    tokens: BTreeMap<(String, String), HoldToken>,
    holds: BTreeMap<(String, String), u32>,
}

impl Drafts {
    #[must_use]
    pub fn new() -> Drafts {
        Drafts::default()
    }

    /// A draft arriving at a room whose version is `current`.
    pub fn submit(&mut self, draft: &Draft, current: Version, turn: u32) -> Submission {
        if draft.seen() == current {
            self.clear(draft);
            return Submission::Delivered;
        }
        let holds = self.hold(draft, current, turn);
        if holds >= DRAFT_HELD_ESCALATE {
            return Submission::Escalated { holds };
        }
        Submission::Held {
            token: self.token(draft).unwrap_or(HoldToken {
                shown: current,
                turn,
            }),
            holds,
        }
    }

    /// The author's answer.
    pub fn resolve(
        &mut self,
        draft: &Draft,
        choice: Return,
        current: Version,
        turn: u32,
    ) -> Resolution {
        match choice {
            Return::Withdraw => {
                self.clear(draft);
                Resolution::Withdrawn
            }
            // Both of these are ordinary resubmissions: they take the
            // same door the draft came in by, and may be held again.
            Return::Rewrite | Return::SendAsIs => match self.submit(draft, current, turn) {
                Submission::Delivered => Resolution::Delivered,
                Submission::Held { token, holds } => Resolution::Held { token, holds },
                Submission::Escalated { holds } => Resolution::Escalated { holds },
            },
            Return::ForceInformed => {
                let live = self
                    .token(draft)
                    .filter(|token| token.turn == turn && token.shown == current);
                match live {
                    Some(_) => {
                        self.clear(draft);
                        Resolution::Delivered
                    }
                    None => {
                        // Void, not refused outright: the author is shown
                        // the room as it stands now and may answer again.
                        self.hold(draft, current, turn);
                        Resolution::TokenVoid {
                            token: HoldToken {
                                shown: current,
                                turn,
                            },
                        }
                    }
                }
            }
        }
    }

    /// How many times in a row this author has been held in this room.
    #[must_use]
    pub fn holds(&self, draft: &Draft) -> u32 {
        self.holds.get(&self.key(draft)).copied().unwrap_or(0)
    }

    /// The `draft_held` record: what the author saw, what the room is at
    /// now, and the four ways back. The body is not repeated - it is the
    /// author's own text and the ledger already carries it once.
    ///
    /// # Errors
    /// Propagates the payload's refusal to hold what it was given.
    pub fn held_payload(&self, draft: &Draft, current: Version) -> Result<Payload, AxError> {
        let mut map = Map::new();
        map.insert(
            "author".to_owned(),
            Value::String(draft.author().to_owned()),
        );
        map.insert(
            "room".to_owned(),
            Value::String(draft.room().as_str().to_owned()),
        );
        map.insert(
            "seen".to_owned(),
            Value::Number(draft.seen().value().into()),
        );
        map.insert("current".to_owned(), Value::Number(current.value().into()));
        map.insert("holds".to_owned(), Value::Number(self.holds(draft).into()));
        map.insert(
            "returns".to_owned(),
            Value::Array(
                [
                    Return::Rewrite,
                    Return::SendAsIs,
                    Return::Withdraw,
                    Return::ForceInformed,
                ]
                .iter()
                .map(|r| Value::String(r.as_str().to_owned()))
                .collect(),
            ),
        );
        Payload::new(map)
    }

    /// The `draft_resolved` record.
    ///
    /// # Errors
    /// Propagates the payload's refusal to hold what it was given.
    pub fn resolved_payload(&self, draft: &Draft, choice: Return) -> Result<Payload, AxError> {
        let mut map = Map::new();
        map.insert(
            "author".to_owned(),
            Value::String(draft.author().to_owned()),
        );
        map.insert(
            "room".to_owned(),
            Value::String(draft.room().as_str().to_owned()),
        );
        map.insert(
            "return".to_owned(),
            Value::String(choice.as_str().to_owned()),
        );
        Payload::new(map)
    }

    fn key(&self, draft: &Draft) -> (String, String) {
        (draft.author().to_owned(), draft.room().as_str().to_owned())
    }

    fn token(&self, draft: &Draft) -> Option<HoldToken> {
        self.tokens.get(&self.key(draft)).copied()
    }

    fn hold(&mut self, draft: &Draft, current: Version, turn: u32) -> u32 {
        let key = self.key(draft);
        let holds = self
            .holds
            .entry(key.clone())
            .and_modify(|n| *n = n.saturating_add(1))
            .or_insert(1);
        self.tokens.insert(
            key,
            HoldToken {
                shown: current,
                turn,
            },
        );
        *holds
    }

    fn clear(&mut self, draft: &Draft) {
        let key = self.key(draft);
        self.tokens.remove(&key);
        self.holds.remove(&key);
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
mod tests;
