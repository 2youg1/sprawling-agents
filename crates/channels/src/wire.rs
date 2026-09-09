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
//! Names of things this crate does not own - modes, providers, templates -
//! travel as validated newtypes with no closed value list. The authority for
//! which values are legal stays upstream (`runtime::Mode`, gateway, city);
//! the mapping point is the assembly layer, and an unknown value is an error
//! there, never a guess.

use kernel::{Address, AxError, B3Hash, EventRecord, RunId, Seq};
use serde::{Deserialize, Serialize};

/// Wire format version. Bumped whenever the frame grammar changes shape in a
/// way the schema hash alone would not explain to a human reading a log.
///
/// 5: `Dispatch` carries the name of the session it starts (F2.11).
/// 6: and how hard that session thinks (F2.16).
/// 7: a provider can be asked what it serves before it is attached, and
///    an attachment names which of those models it admits (P3.01).
/// 8: a building's sandbox limits and external servers have a surface
///    (P3.02).
/// 9: a page can ask for the history that happened before it opened
///    (P3.04).
/// 12: a third class of frame carries what a model is saying while it is
///    still saying it (V3.13). It is not an event: it has no sequence
///    number, it is never written down, and a client that missed one has
///    lost nothing.
/// 13: the plan is a tree, so a building's answer carries its nodes,
///    what each is worth and what is ready; a branch that is stuck says
///    so once, at the node it is stuck at; and a city can be given a
///    goal it works towards until the work runs out (V3.17-V3.23).
/// 14: a commit the city made can be asked which run wrote it
///    (card-2.4).
/// 15: that answer carries the lineage of the run - the successors
///    a resident replaced itself through (card-11.6).
/// 16: three readings a page used to compute for itself are questions
///    the server answers - a session's rounds, what a run left as
///    evidence, and what one plan node cost (card-6.5).
/// 17: the first client that only asks the wire found four gaps in it -
///    a run's room and start, a session's opening and closing, the
///    scopes a halt shut, and the two questions that walk the tree,
///    `Listing` and `Document` (card-6.4).
pub const WIRE_V: u32 = 17;
mod query;

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
    B3Hash::digest(&material)
}

/// The JSON Schema of the whole envelope: every named type reachable from
/// [`ClientFrame`] or [`ServerFrame`], under `$defs`, both roots included.
///
/// This is what `cargo xtask wire-ts` generates the client from. It is
/// not what the handshake compares: [`schema_hash`] reads the version and
/// the two name tables and nothing else, so a doc comment edited here
/// moves this document and leaves every connected page connected. Pure:
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
    /// Where the Event stream resumes, so a reconnect leaves no gap.
    pub resume_from: Option<Seq>,
    /// Which city answered. The handshake is where a connection learns
    /// whose city it is: the name is in the Ledger's first record, and a
    /// client that only ever hears what happens *next* would otherwise
    /// have to display "no city" over a city that has been running for a
    /// month.
    pub city: Option<Address>,
}

/// Everything a client may send.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ClientFrame {
    Hello(Hello),
    Command(Box<WireCommand>),
    Query(Query),
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
    Answer(Box<Answer>),
    Refusal(Box<AxError>),
    /// Text a model is saying, before the call it belongs to has
    /// settled. Discardable by construction: the run it belongs to is
    /// named so a client can throw the buffer away when `model_returned`
    /// arrives, and the settled text of that record is what a page
    /// draws. Where the two disagree, the record wins.
    Delta(Delta),
}

/// One piece of what a model is saying, on its way to a page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Delta {
    pub run: RunId,
    pub text: String,
}
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn the_query_names_match_the_variants() {
        let queries = [
            Query::History {
                before: None,
                limit: 20,
            },
            Query::RunHistory {
                run: RunId::from_bytes([1u8; 16]),
                before: None,
                limit: 20,
            },
            Query::Changes {
                base: kernel::GitOid::from_bytes([2u8; 20]),
                head: None,
            },
            Query::Hunks {
                oid_a: kernel::GitOid::from_bytes([4u8; 20]),
                oid_b: kernel::GitOid::from_bytes([5u8; 20]),
                path: "lab/lex.rs".to_owned(),
            },
            Query::Commit {
                oid: kernel::GitOid::from_bytes([3u8; 20]),
            },
            Query::RunView {
                run: RunId::from_bytes([1u8; 16]),
            },
            Query::CityView,
            Query::ApprovalQueue,
            Query::InboxView {
                addr: Address::parse("acme").unwrap(),
            },
            Query::Metrics,
            Query::CostView,
            Query::ArchiveSearch {
                needle: "x".to_owned(),
            },
            Query::RegistryView,
            Query::DiscardView,
            Query::EndpointView,
            Query::BuildingView {
                addr: Address::parse("acme").unwrap(),
            },
            Query::Governance,
            Query::Rounds {
                run: RunId::from_bytes([1u8; 16]),
            },
            Query::Evidence {
                run: RunId::from_bytes([1u8; 16]),
            },
            Query::CostOf {
                node: kernel::NodeId::parse("2.3").unwrap(),
            },
            Query::Listing { at: None },
            Query::Document {
                at: Address::parse("acme/Roadmap.md").unwrap(),
            },
        ];
        assert_eq!(queries.len(), QUERY_NAMES.len());
        for (query, expected) in queries.iter().zip(QUERY_NAMES) {
            assert_eq!(query.name(), expected, "declaration order must match");
        }
    }

    #[test]
    fn a_client_frame_round_trips_through_json() {
        let frame = ClientFrame::Query(Query::CityView);
        let text = serde_json::to_string(&frame).unwrap();
        let back: ClientFrame = serde_json::from_str(&text).unwrap();
        assert_eq!(frame, back);
    }

    /// The document names both roots, every command by its wire name,
    /// and the one frame a socket cannot spell as a value nothing
    /// satisfies — so the client generated from it refuses the same
    /// bytes the server refuses.
    #[cfg(feature = "schema")]
    #[test]
    fn the_schema_document_holds_both_roots_and_every_command() {
        let document = wire_schema();
        let defs = document.get("$defs").and_then(|d| d.as_object()).unwrap();
        assert!(defs.contains_key("ClientFrame"), "client root");
        assert!(defs.contains_key("ServerFrame"), "server root");
        let command = serde_json::to_string(defs.get("Command").unwrap()).unwrap();
        for name in COMMAND_NAMES {
            let mut snake = String::new();
            for (index, ch) in name.chars().enumerate() {
                if ch.is_ascii_uppercase() && index > 0 {
                    snake.push('_');
                }
                snake.push(ch.to_ascii_lowercase());
            }
            assert!(
                command.contains(&format!("\"{snake}\"")),
                "{name} on the wire"
            );
        }
        assert!(
            defs.get("NoSecret") == Some(&serde_json::Value::Bool(false)),
            "a credential over the wire satisfies nothing"
        );
        assert_eq!(wire_schema(), document, "the document is a pure function");
    }
}
