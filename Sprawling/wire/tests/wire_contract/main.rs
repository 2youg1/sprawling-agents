// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

#[cfg(feature = "server")]
use std::net::SocketAddr;

#[cfg(feature = "server")]
use kernel::AxCode;
use kernel::{Address, Sealed, Seq};
#[cfg(feature = "server")]
use wire::{BindFace, BindVerdict, HandshakeVerdict, decide_bind, decide_handshake};
use wire::{COMMAND_NAMES, Command, Mode, QUERY_NAMES, Query, WIRE_V, schema_hash};
#[cfg(feature = "server")]
use wire::{Hello, Welcome};

mod samples;

use samples::sample_of_every_command;

#[cfg(feature = "server")]
fn loopback() -> SocketAddr {
    "127.0.0.1:2501".parse().unwrap()
}

#[cfg(feature = "server")]
fn exposed() -> SocketAddr {
    "192.168.1.20:2501".parse().unwrap()
}

#[test]
fn the_command_and_query_tables_hold_their_declared_counts() {
    assert_eq!(COMMAND_NAMES.len(), 50, "command table");
    assert_eq!(QUERY_NAMES.len(), 64, "query table");

    let mut sorted = COMMAND_NAMES.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 50, "command names are distinct");

    let mut sorted = QUERY_NAMES.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 64, "query names are distinct");
}

#[test]
fn every_command_name_is_registered_in_the_table() {
    for command in sample_of_every_command() {
        let name = command.name();
        assert!(
            COMMAND_NAMES.contains(&name),
            "command `{name}` is missing from COMMAND_NAMES"
        );
    }
}

#[test]
fn the_schema_hash_is_stable_across_calls_and_covers_the_wire_version() {
    assert_eq!(schema_hash(), schema_hash(), "hash is a pure function");

    assert_eq!(
        schema_hash().to_string(),
        WIRE_SCHEMA_GOLDEN,
        "schema hash changed - update `Sprawling/wire/spec/Frames.lean` §8-1 in the same commit"
    );
    assert_eq!(
        WIRE_V, 81,
        "WIRE_V rises once between two pushes, for a shape change under names that stay (wire D1)"
    );
}

#[test]
fn the_schema_hash_covers_every_event_kind_name() {
    let mut material = b"sprawling/wire/".to_vec();
    material.extend_from_slice(&WIRE_V.to_le_bytes());
    let kinds = kernel::EventKind::ALL.map(|kind| format!("{kind:?}"));
    let tagged = COMMAND_NAMES.iter().map(|name| (b'C', *name));
    let tagged = tagged.chain(QUERY_NAMES.iter().map(|name| (b'Q', *name)));
    for (tag, name) in tagged.chain(kinds.iter().map(|name| (b'E', name.as_str()))) {
        material.push(tag);
        material.extend_from_slice(name.as_bytes());
    }
    assert_eq!(schema_hash(), kernel::B3Hash::digest(&material));
}

const WIRE_SCHEMA_GOLDEN: &str = "13c28a66810d6e35a9b477ccd507c25e458a39811c5fa86e6f574ee331dfd593";

#[cfg(feature = "schema")]
#[test]
fn the_wire_shape_is_pinned_so_a_change_meets_the_version_rule() {
    let mut shape = wire::wire_schema();
    strip_prose(&mut shape);
    let digest = kernel::B3Hash::digest(&serde_json::to_vec(&shape).unwrap());
    assert_eq!(
        digest.to_string(),
        WIRE_SHAPE_GOLDEN,
        "the wire changed shape. If no shape change has landed since the last push, raise \
         WIRE_V in this commit (wire D1); then set WIRE_SHAPE_GOLDEN to the digest above"
    );
}

#[cfg(feature = "schema")]
fn strip_prose(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            map.retain(|key, held| !((key == "description" || key == "title") && held.is_string()));
            map.values_mut().for_each(strip_prose);
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(strip_prose),
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => {}
    }
}

#[cfg(feature = "schema")]
const WIRE_SHAPE_GOLDEN: &str = "d2642389c18f2f4664543ac9dc549dbf0b67476800ff96e569dd1c975115c5cf";

#[cfg(feature = "server")]
#[test]
fn the_binding_face_refuses_exactly_when_no_key_is_given() {
    let secret = kernel::B3Hash::digest(b"pair-me-during-binding");
    for addr in [loopback(), exposed()] {
        let BindVerdict::Refuse(err) = decide_bind(&addr, None) else {
            panic!("{addr} with no key must refuse to start");
        };
        assert_eq!(*err.code(), AxCode::ConfigInvalid);
        assert!(
            !err.recovery().is_empty(),
            "a refusal names an executable alternative"
        );
        let BindVerdict::Serve(face) = decide_bind(&addr, Some(secret)) else {
            panic!("{addr} with a key is served");
        };
        assert_eq!(face.key(), &secret);
    }
    assert!(matches!(
        decide_bind(&loopback(), Some(secret)),
        BindVerdict::Serve(BindFace::Loopback { .. })
    ));
    assert!(matches!(
        decide_bind(&exposed(), Some(secret)),
        BindVerdict::Serve(BindFace::Exposed { .. })
    ));
}

#[cfg(feature = "server")]
#[test]
fn a_mismatched_schema_hash_is_rejected_before_anything_else() {
    let good = Hello {
        wire_v: WIRE_V,
        schema: schema_hash(),
        token: Some(LOOSE_KEY.to_owned()),
    };
    let expected = Welcome {
        wire_v: WIRE_V,
        schema: schema_hash(),
        resume_from: Some(Seq::new(7)),
        city: None,
        epoch: None,
    };
    assert!(matches!(
        decide_handshake(&good, &expected, &loose()),
        HandshakeVerdict::Accept
    ));

    let stale = Hello {
        wire_v: WIRE_V,
        schema: kernel::B3Hash::digest(b"a client that cached an older front end"),
        token: None,
    };
    let HandshakeVerdict::Reject(err) = decide_handshake(&stale, &expected, &loose()) else {
        panic!("a cached older client must be told to refresh, not silently served");
    };
    assert_eq!(*err.code(), AxCode::WireMismatch);
}

#[cfg(feature = "server")]
const LOOSE_KEY: &str = "native-key-0123456789";

#[cfg(feature = "server")]
fn loose() -> wire::Keys<'static> {
    keys_of(BindFace::Loopback {
        key: kernel::B3Hash::digest(LOOSE_KEY.as_bytes()),
    })
}

#[cfg(feature = "server")]
fn keys_of(face: BindFace) -> wire::Keys<'static> {
    wire::Keys {
        face: Box::leak(Box::new(face)),
        sessions: Box::leak(Box::default()),
        now: kernel::TimeMs::new(0),
    }
}

#[cfg(feature = "server")]
#[test]
fn an_exposed_server_rejects_a_wrong_token_and_accepts_the_right_one() {
    let expected = Welcome {
        wire_v: WIRE_V,
        schema: schema_hash(),
        resume_from: None,
        city: None,
        epoch: None,
    };
    let secret = kernel::B3Hash::digest(b"pair-me-0123456789");
    let paired = keys_of(BindFace::Exposed { key: secret });

    let wrong = Hello {
        wire_v: WIRE_V,
        schema: schema_hash(),
        token: Some("pair-me-9876543210".to_owned()),
    };
    assert!(matches!(
        decide_handshake(&wrong, &expected, &paired),
        HandshakeVerdict::Reject(_)
    ));

    let right = Hello {
        wire_v: WIRE_V,
        schema: schema_hash(),
        token: Some("pair-me-0123456789".to_owned()),
    };
    assert!(matches!(
        decide_handshake(&right, &expected, &paired),
        HandshakeVerdict::Accept
    ));

    let absent = Hello {
        wire_v: WIRE_V,
        schema: schema_hash(),
        token: None,
    };
    assert!(
        matches!(
            decide_handshake(&absent, &expected, &paired),
            HandshakeVerdict::Reject(_)
        ),
        "a missing token is not an empty token"
    );

    assert!(matches!(
        decide_handshake(&absent, &expected, &loose()),
        HandshakeVerdict::Reject(_)
    ));
}

#[test]
fn put_secret_has_no_byte_form_in_either_direction() {
    let json = r#"{"put_secret":{"realm":"anthropic","name":"api","value":"sk-not-a-real-key"}}"#;
    let decoded: Result<wire::WireCommand, _> = serde_json::from_str(json);
    let err = decoded.expect_err("PutSecret has no wire form");
    assert!(
        !err.to_string().contains("sk-not-a-real-key"),
        "the refusal must not echo the very bytes it is protecting"
    );

    let local = Command::PutSecret {
        realm: "anthropic".to_owned(),
        name: "api".to_owned(),
        value: Sealed::new(Box::new("sk-not-a-real-key".to_owned())),
    };
    assert_eq!(local.name(), "PutSecret");
    assert!(local.idem().is_none());
}

#[test]
fn every_state_changing_command_carries_an_idempotency_key() {
    for command in sample_of_every_command() {
        let name = command.name();
        if name == "Auth" || name == "PutSecret" {
            assert!(
                command.idem().is_none(),
                "`{name}` changes nothing a replay could double"
            );
        } else {
            assert!(
                command.idem().is_some(),
                "state-changing command `{name}` has no IdemKey"
            );
        }
    }
}

#[test]
fn a_frame_that_left_the_wire_fails_to_decode() {
    let run = kernel::RunId::from_bytes([7u8; 16]);
    let idem =
        serde_json::to_value(kernel::IdemKey::derive(&run, Seq::new(1), b"retired")).unwrap();
    let checkpoint = serde_json::to_value(kernel::GitOid::from_bytes([0x5au8; 20])).unwrap();
    for frame in [
        serde_json::json!({ "takeover": { "run": run.to_string(), "idem": idem.clone() } }),
        serde_json::json!({ "rollback": { "checkpoint": checkpoint, "idem": idem } }),
    ] {
        let decoded: Result<Command<wire::NoSecret>, _> = serde_json::from_value(frame.clone());
        let refused = decoded.expect_err("a frame that left the wire is not a command");
        assert!(
            refused.to_string().contains("unknown variant"),
            "the refusal is the grammar's, not something weaker: {refused} in {frame}"
        );
    }
}

#[test]
fn a_session_frame_says_what_it_branches_from_and_omits_it_when_it_does_not() {
    let run = kernel::RunId::from_bytes([9u8; 16]);
    let idem = kernel::IdemKey::derive(&run, Seq::new(1), b"branch");
    let room = Address::parse("acme/floor1").unwrap();
    let plain = serde_json::to_value(Command::<wire::NoSecret>::OpenSession {
        addr: room.clone(),
        carry: wire::Carry::Nothing,
        from: None,
        idem,
    })
    .unwrap();
    assert_eq!(
        plain["open_session"]["from"],
        serde_json::Value::Null,
        "a session that branches from nothing says null: {plain}"
    );

    let branching = serde_json::to_value(Command::<wire::NoSecret>::OpenSession {
        addr: room,
        carry: wire::Carry::Nothing,
        from: Some(kernel::Origin {
            run,
            at_seq: Seq::new(7),
        }),
        idem,
    })
    .unwrap();
    assert_eq!(branching["open_session"]["from"]["at_seq"], 7);
    assert_eq!(branching["open_session"]["from"]["run"], run.to_string());
}

#[test]
fn the_command_sample_covers_the_whole_table() {
    let sample = sample_of_every_command();
    assert_eq!(
        sample.len(),
        COMMAND_NAMES.len(),
        "the fixture must exercise every command"
    );
}

#[test]
fn a_query_never_carries_an_idempotency_key() {
    for name in QUERY_NAMES {
        assert!(!name.is_empty());
    }
    assert!(matches!(Query::ApprovalQueue, Query::ApprovalQueue));
}

#[test]
fn asking_for_one_session_is_a_different_frame_from_asking_for_the_city() {
    let run = kernel::RunId::from_bytes([4u8; 16]);
    let mine = Query::RunHistory {
        run,
        before: Some(kernel::Seq::new(90)),
        limit: 50,
    };
    let bytes = serde_json::to_vec(&mine).expect("a query serialises");
    let back: Query = serde_json::from_slice(&bytes).expect("and reads back");
    assert_eq!(back, mine);
    assert_eq!(mine.name(), "RunHistory");
    assert!(QUERY_NAMES.contains(&mine.name()), "named in the table");

    let city = Query::History {
        before: Some(kernel::Seq::new(90)),
        limit: 50,
    };
    assert_ne!(
        serde_json::to_vec(&city).expect("a query serialises"),
        bytes,
        "one session and the whole city must not spell the same frame"
    );
}

#[test]
fn a_skipped_range_travels_with_both_of_its_ends() {
    let frame = wire::ServerFrame::Lagged(wire::Lagged {
        from: Seq::new(12),
        to: Seq::new(40),
    });
    let text = serde_json::to_string(&frame).expect("a frame serialises");
    let back: wire::ServerFrame = serde_json::from_str(&text).expect("and reads back");
    assert_eq!(back, frame, "both ends survive the wire");
    assert!(
        text.contains("\"from\":12") && text.contains("\"to\":40"),
        "{text}"
    );

    let ask = Query::HistoryRange {
        from: Seq::new(12),
        to: Seq::new(40),
        limit: 500,
    };
    assert_eq!(ask.name(), "HistoryRange");
    assert!(QUERY_NAMES.contains(&ask.name()), "named in the table");
    let bytes = serde_json::to_vec(&ask).expect("a query serialises");
    let back: Query = serde_json::from_slice(&bytes).expect("and reads back");
    assert_eq!(back, ask);
    assert_ne!(
        bytes,
        serde_json::to_vec(&Query::History {
            before: Some(Seq::new(40)),
            limit: 500,
        })
        .expect("a query serialises"),
        "a range and a backward page must not spell the same frame"
    );
}

#[test]
fn a_dispatch_frame_carries_no_spend_ceiling() {
    let addr = Address::parse("acme/floor1").unwrap();
    let run = kernel::RunId::from_bytes([7u8; 16]);
    let dispatch: wire::WireCommand = Command::Dispatch {
        addr,
        task: "ship it".to_owned(),
        goal: "the tests pass".to_owned(),
        policy: wire::RunPolicy::of(Mode::Work),
        idem: kernel::IdemKey::derive(&run, Seq::new(1), b"sample"),
        session: None,
        effort: None,
        model: None,
    };
    let text = serde_json::to_string(&dispatch).unwrap();
    assert!(
        !text.contains("budget"),
        "a dispatch frame states no ceiling: {text}"
    );
}

#[test]
fn the_governance_frames_are_on_the_wire() {
    assert!(
        COMMAND_NAMES.contains(&"PutDocument"),
        "a person can write the documents that govern their city"
    );
    assert!(
        QUERY_NAMES.contains(&"Governance"),
        "and read who answers and what was answered for them"
    );
    let frame: wire::WireCommand = Command::PutDocument {
        which: wire::GovernedDocument::Preferences,
        base: String::new(),
        body: "# how I like this city run\n".to_owned(),
        idem: kernel::IdemKey::derive(&kernel::RunId::from_bytes([9u8; 16]), Seq::new(1), b"prefs"),
    };
    assert_eq!(frame.name(), "PutDocument");
    let text = serde_json::to_string(&frame).unwrap();
    let back: wire::WireCommand = serde_json::from_str(&text).unwrap();
    assert_eq!(back, frame);
    assert_eq!(Query::Governance.name(), "Governance");
}

#[test]
fn one_files_patch_is_a_frame_of_its_own() {
    assert!(
        QUERY_NAMES.contains(&"Hunks"),
        "reviewing a change in a browser needs the patch text of one file"
    );
    let query = Query::Hunks {
        oid_a: kernel::GitOid::from_bytes([1u8; 20]),
        oid_b: kernel::GitOid::from_bytes([2u8; 20]),
        path: "lab/lex.rs".to_owned(),
    };
    assert_eq!(query.name(), "Hunks");
    let bytes = serde_json::to_vec(&query).unwrap();
    let back: Query = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(back, query);

    assert_ne!(
        bytes,
        serde_json::to_vec(&Query::Changes {
            base: kernel::GitOid::from_bytes([1u8; 20]),
            head: Some(kernel::GitOid::from_bytes([2u8; 20])),
        })
        .unwrap()
    );
}
