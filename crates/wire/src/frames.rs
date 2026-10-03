// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The process boundary's vocabulary: the Commands, the Queries, and
//! the Event push. Encoding is JSON because the receiving
//! end is a browser and a human reading a network panel is a design goal.
//!
//! Two invariants live in the type system rather than in a check:
//!
//! - `Command` is generic over the carrier of a secret. `WireCommand` fixes
//!   that carrier to an uninhabited type, so a frame arriving from a socket
//!   cannot be a `PutSecret` - not "is rejected", but has no representation.
//!   Credentials are enrolled on the host machine, and that constraint is
//!   held by construction rather than by a check.
//! - Every state-changing Command owns an `IdemKey` field. There is no
//!   constructor that omits it, so "double-clicking twice opens two Runs" is
//!   not reachable from this type.
//!
//! Names of things this crate does not own - providers, templates, toolkits -
//! travel as validated newtypes with no closed value list. The authority for
//! which values are legal stays upstream (gateway, city, the broker);
//! the mapping point is the assembly layer, and an unknown value is an error
//! there, never a guess.

use kernel::{Address, AxError, B3Hash, EventKind, EventRecord, RunId, Seq, TimeMs};
use serde::{Deserialize, Serialize};

/// Wire format version. Rises by one between two pushes, in the first commit
/// that changes a frame's shape while every frame and kind name stays
/// (wire D1); a changed name moves [`schema_hash`] on its own.
pub const WIRE_V: u32 = 54;
mod ask;
mod monitor;
/// Queries read state. They are cacheable and free of side effects, so
/// none carries an `IdemKey`: a Query that needed one would have stopped
/// being a Query. The name table is generated from the variant list by
/// `named_frames!`, because the schema hash is built from it and a table
/// that could drift from the enum would let two builds agree on a hash
/// while disagreeing on what a frame means.
mod query;

pub use ask::{Answered, Ask, AskId, AskOutcome};
pub use monitor::{BEAT_MAX_MS, BEAT_MIN_MS, BeatMs, Monitoring, Sample, Watched};
pub use query::{QUERY_NAMES, Query};

use crate::answer::Answer;
use crate::command::{COMMAND_NAMES, WireCommand};

/// A digest of the protocol surface, exchanged at connect time.
///
/// A browser can hold a cached older client while the server has moved on;
/// that mismatch is the one error WebUI has that a native window does not,
/// and this hash is its single answer. Pure: same inputs, same bytes, always.
#[must_use]
pub fn schema_hash() -> B3Hash {
    let mut material = Vec::new();
    material.extend_from_slice(b"sprawling/wire/");
    material.extend_from_slice(&WIRE_V.to_le_bytes());
    for name in COMMAND_NAMES {
        material.push(b'C');
        material.extend_from_slice(name.as_bytes());
    }
    for name in QUERY_NAMES {
        material.push(b'Q');
        material.extend_from_slice(name.as_bytes());
    }
    for kind in EventKind::ALL {
        material.push(b'E');
        material.extend_from_slice(format!("{kind:?}").as_bytes());
    }
    B3Hash::digest(&material)
}

/// The JSON Schema of the whole envelope: every named type reachable from
/// [`ClientFrame`] or [`ServerFrame`], under `$defs`, both roots included.
///
/// This is what `cargo xtask wire-ts` generates the client from. It is
/// not what the handshake compares: [`schema_hash`] reads the version,
/// the two frame name tables and the event kind names and nothing else,
/// so a doc comment edited here moves this document and leaves every
/// connected page connected. Pure:
/// same build, same bytes.
#[cfg(feature = "schema")]
#[must_use]
pub fn wire_schema() -> serde_json::Value {
    let mut generator = schemars::SchemaGenerator::default();
    generator.subschema_for::<ClientFrame>();
    generator.subschema_for::<ServerFrame>();
    let definitions = generator.take_definitions(true);
    serde_json::json!({
        "$schema": schemars::consts::meta_schemas::DRAFT2020_12,
        "title": "sprawling wire",
        "$defs": definitions,
    })
}

/// The client's opening frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Hello {
    pub wire_v: u32,
    pub schema: B3Hash,
    /// Present only when the server binds a non-loopback address.
    pub token: Option<String>,
}

/// The server's answer to a `Hello` it accepted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Welcome {
    pub wire_v: u32,
    pub schema: B3Hash,
    /// The seq of the last record the city had broadcast when this welcome
    /// was sent. Every later record follows on the live stream, so a
    /// client that reconnects fetches the records after its own mark up to
    /// and including this seq; a record at the boundary may arrive twice,
    /// and none arrives not at all.
    pub resume_from: Option<Seq>,
    /// Which city answered. The handshake is where a connection learns
    /// whose city it is: the name is in the Ledger's first record, and a
    /// client that only ever hears what happens *next* would otherwise
    /// have to display "no city" over a city that has been running for a
    /// month.
    pub city: Option<Address>,
    /// Which ledger answered: the chain hash of its first line. A client
    /// whose mark came from a welcome with another epoch holds positions
    /// in a different history, and rebuilds rather than resumes.
    pub epoch: Option<B3Hash>,
}

/// Everything a client may send.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ClientFrame {
    Hello(Hello),
    Command(Box<WireCommand>),
    Ask(Ask),
    Monitor(Monitoring),
}

/// Everything a server may send. Events are the push half; a `Refusal`
/// carries the three-part refusal the interface renders verbatim.
///
/// **`Delta` is deliberately not an event.** An event is a thing that
/// happened, and a token increment is not: it has no sequence number, it
/// is never written to the Ledger, it cannot be replayed, and a client
/// that missed one has lost nothing. Giving it a frame class of its own
/// is what keeps that true — folded into the event stream it would
/// become a second, unverifiable history of what the model said.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ServerFrame {
    Welcome(Welcome),
    Event(Box<EventRecord>),
    Answered(Box<Answered>),
    Refusal(Box<AxError>),
    /// Text a model is saying, before the call it belongs to has
    /// settled. Discardable by construction: the run it belongs to is
    /// named so a client can throw the buffer away when `model_returned`
    /// arrives, and the settled text of that record is what a page
    /// draws. Where the two disagree, the record wins.
    Delta(Delta),
    /// One line of the process log, as `docs/logging.md` defines it.
    ///
    /// **A log is not history**, and this frame is what keeps that
    /// true while a person still gets to read it: the line has no
    /// ledger sequence of its own, it is never written down, and a
    /// client that missed one has lost nothing. It carries the ledger
    /// position it was written at, which is the integer the two
    /// timelines line up on.
    Log(LogLine),
    /// The ledger records this session's event stream skipped.
    ///
    /// A session that reads slower than the city writes loses the
    /// middle of the stream: the buffer behind a subscription has a
    /// fixed capacity, and one slow reader is left behind rather than
    /// holding the writer up. Losing it in silence would let a page draw
    /// a history with a hole in it while nothing said so. The
    /// range is answered from the Ledger, which is the one home of what
    /// happened.
    ///
    /// **Only the event stream states a gap.** An increment and a log
    /// line are written down nowhere, so a range naming them would name
    /// records that do not exist, and a reader that missed one has lost
    /// nothing it could have acted on.
    Lagged(Lagged),
    /// What a command still running has written so far. Discardable by
    /// the rule `Delta` follows: the call's result in the Ledger is the
    /// authority on that output, and a page drops this once it lands.
    Output(LiveOutput),
    /// One monitor reading, sent only to a session that is watching.
    Monitor(Sample),
}

/// Which of a command's two outputs a piece came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum OutputStream {
    Out,
    Err,
}

/// One piece of a running command's output, on its way to a page.
///
/// `text` is decoded lossily: a piece ends at a byte bound, which can
/// fall inside a character, and the settled result is what a page keeps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct LiveOutput {
    pub run: RunId,
    pub stream: OutputStream,
    pub text: String,
}

/// Ledger records that never reached a peer, named by both ends.
///
/// Both ends come from records this session can name, never from the
/// count a lagged subscription reports: that count says how many
/// messages were skipped and neither endpoint, so a reader holding it
/// cannot ask for the range it lost. `from` is the record after the last
/// one the session delivered; `to` is the record before the first one to
/// arrive after the gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Lagged {
    pub from: Seq,
    pub to: Seq,
}

/// One piece of what a model is producing, on its way to a page.
///
/// The piece says which of the two streams it came from, because a page
/// draws them differently: prose is the answer and reasoning is folded
/// away beside it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Delta {
    pub run: RunId,
    pub increment: kernel::Increment,
}

/// Who reads a log line, and when.
///
/// The five `docs/logging.md` names, spelled on the wire exactly as
/// `runtime::diagnostics::Level` spells them on a terminal. This crate
/// cannot depend on `runtime` — the graph points inward — so the
/// mapping between the two lives at the assembly layer, where a test
/// holds the two spellings equal name for name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum LogLevel {
    Refuse,
    Effect,
    Decide,
    Trace,
    Wire,
}

/// One diagnostic line on its way to a page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct LogLine {
    /// Where the ledger stood when the line was written. Not a sequence
    /// of this stream's own: two lines can share it, and a reader uses
    /// it to put the line beside the history rather than to detect a
    /// gap.
    pub seq: Seq,
    /// When this machine wrote it. Absent when the clock could not be
    /// read, because `seq` is the anchor either way and dropping the
    /// line would lose the diagnostic to save the timestamp.
    pub t: Option<TimeMs>,
    pub level: LogLevel,
    pub module: String,
    /// Which run it was written under, and absent for the city itself.
    pub run: Option<RunId>,
    pub line: String,
}
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
#[path = "frames/tests.rs"]
mod tests;
