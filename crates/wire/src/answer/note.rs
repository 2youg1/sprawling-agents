// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a turn came to besides its calls (`crates/wire/spec/Reading.lean`
//! §8-21, D36-D38).

use kernel::event::record::SignalKind;
use kernel::{Address, AxError, GitOid, RunId, Seq, TimeMs};
use serde::{Deserialize, Serialize};

/// What a turn came to besides the calls it made.
///
/// **The criterion is closed on purpose**: an event earns a `Note` when
/// it changed what this turn did, or what it is waiting on. Everything
/// else stays in the event stream, which is the Ledger's shape rather
/// than a reader's. Without that line this enum would grow to one arm
/// per event kind and stop meaning anything.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Note {
    /// A door refused something. The error travels whole because the
    /// interface has one place where a refusal becomes the three parts a
    /// person needs; taking it apart here would be the second.
    Refused { error: AxError, at: Seq },
    /// A checkpoint went up, and this is the commit it made. It is
    /// what a change list is addressed by.
    Checkpointed { oid: GitOid, at: Seq },
    /// This turn stopped for a person. What waits and who answers is the
    /// approval queue's; copying it here would be a third authority.
    /// `t` is when the Ledger recorded the request and `answered` when
    /// it recorded the answer, paired back by approval id from the
    /// city's own run; `None` when the answer is outside that window or
    /// has not come, and the page then draws no guessed end.
    Waiting {
        at: Seq,
        t: TimeMs,
        answered: Option<TimeMs>,
    },
    /// A word arrived - from the person watching, or from another
    /// address that reached this one (D36). `t` is when this session
    /// took it. A pulled signal's sender and words sit on its sending
    /// line, under the sender's run, so `from` and `said` are `None`
    /// until the server's fold pairs that line by its id, and stay
    /// `None` when the sending is outside the window it searches.
    Arrived {
        from: Option<String>,
        said: Option<String>,
        by: Speaker,
        t: TimeMs,
        /// Present when the signal is a child session handing its work
        /// back (D38).
        handback: Option<HandbackNote>,
        /// The kind the letter was sent as, from its sending line; `None`
        /// for a steer, which is no letter, and when the sending was not
        /// paired (D43).
        #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
        kind: Option<SignalKind>,
        /// The session that sent the letter: the run its sending line was
        /// written under, which for a handback is the child's. `None` in
        /// the same cases as `kind` (D43).
        session: Option<RunId>,
        at: Seq,
    },
    /// This turn sent with `wait` and stopped until a reply from `on`
    /// or `until` (D37). `ended` is `None` while it waits, and when its
    /// end is outside the window.
    AwaitingReply {
        on: Address,
        until: TimeMs,
        t: TimeMs,
        ended: Option<ReplyEnded>,
        at: Seq,
    },
    /// Files went away. Every one carries its way back, which is the
    /// Recycle Bin's to state.
    Discarded { count: usize, at: Seq },
    /// A record of a kind that earns a note, whose payload did not read
    /// back as that kind. The failure stays visible here instead of the
    /// turn reading as if nothing happened; `cause` names the kind and
    /// what the reading stopped at.
    Unreadable { cause: String, at: Seq },
}

impl Note {
    /// Where in the Ledger this note is, so every row can be read
    /// further.
    #[must_use]
    pub fn at(&self) -> Seq {
        match *self {
            Self::Refused { at, .. }
            | Self::Checkpointed { at, .. }
            | Self::Waiting { at, .. }
            | Self::Arrived { at, .. }
            | Self::AwaitingReply { at, .. }
            | Self::Discarded { at, .. }
            | Self::Unreadable { at, .. } => at,
        }
    }
}

/// Who spoke a word that arrived: the person who owns the city, through
/// a steer; the city itself, through a steer; or a resident, through a
/// steer or a signal (D36, D41).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Speaker {
    User,
    City,
    Resident,
}

/// How a child session's handed-back work ended, read through
/// `collab::Handback::from_signal`; which session it was is
/// [`Note::Arrived`]'s `session` (D38, D43).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum HandbackNote {
    Finished { verified_by: String },
    Stopped { because: String },
}

/// The end of a reply wait, and when the Ledger recorded it (D37).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ReplyEnded {
    pub by: ReplyEnd,
    pub t: TimeMs,
}

/// What ended a reply wait, one arm per arm of `kernel`'s `WaitEnd`;
/// the reply itself is an `Arrived` note of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ReplyEnd {
    Reply,
    Timeout,
    Left,
}
