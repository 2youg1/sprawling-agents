// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The wire contract the rest of the system is allowed to rely on:
//! the command and query counts, the schema hash that gates a connection,
//! the binding decision, and the two shapes that must stay unspellable.

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

// The binding and handshake decisions belong to the listener, so this
// file asserts them only in a build that has one.
#[cfg(feature = "server")]
use kernel::AxCode;
use kernel::{Address, Sealed, Seq};
#[cfg(feature = "server")]
use wire::{BindFace, BindVerdict, HandshakeVerdict, decide_bind, decide_handshake};
use wire::{COMMAND_NAMES, Command, Mode, ProviderName, QUERY_NAMES, Query, WIRE_V, schema_hash};
#[cfg(feature = "server")]
use wire::{Hello, Welcome};

#[cfg(feature = "server")]
fn loopback() -> SocketAddr {
    "127.0.0.1:8787".parse().unwrap()
}

#[cfg(feature = "server")]
fn exposed() -> SocketAddr {
    "192.168.1.20:8787".parse().unwrap()
}

// ---------------------------------------------------------------- wire counts

#[test]
fn the_command_and_query_tables_hold_their_declared_counts() {
    // The counts are the wire's closed surface, not a style choice.
    assert_eq!(COMMAND_NAMES.len(), 29, "command table");
    assert_eq!(QUERY_NAMES.len(), 38, "query table");

    let mut sorted = COMMAND_NAMES.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 29, "command names are distinct");

    let mut sorted = QUERY_NAMES.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 38, "query names are distinct");
}

#[test]
fn every_command_name_is_registered_in_the_table() {
    // `Command::name` is an exhaustive match, so a new variant cannot compile
    // without appearing here; this test closes the other half - it must also
    // appear in the name table that feeds the schema hash.
    for command in sample_of_every_command() {
        let name = command.name();
        assert!(
            COMMAND_NAMES.contains(&name),
            "command `{name}` is missing from COMMAND_NAMES"
        );
    }
}

// ------------------------------------------------------------- schema hashing

#[test]
fn the_schema_hash_is_stable_across_calls_and_covers_the_wire_version() {
    assert_eq!(schema_hash(), schema_hash(), "hash is a pure function");
    // Golden: this pins the current wire. Changing a variant changes the hash,
    // which forces the SPEC to move in the same change set (apisync gate).
    // A changed name moves the hash the handshake compares; `WIRE_V` rises
    // only for a shape change under names that stay, once between two
    // pushes (wire-SPEC §12.1).
    assert_eq!(
        schema_hash().to_string(),
        WIRE_SCHEMA_GOLDEN,
        "schema hash changed - update wire-SPEC.md section 8-1 in the same commit"
    );
    assert_eq!(
        WIRE_V, 45,
        "WIRE_V rises once between two pushes, for a shape change under names that stay (wire-SPEC 12.1)"
    );
}

/// The event kinds reach a page inside every event frame, so a renamed,
/// added or removed kind must move the hash a page is admitted by, as a
/// renamed frame does. The recipe is the one wire-SPEC.md states.
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

/// A function of WIRE_V, the two frame name tables and the event kind
/// names, so any change to the protocol surface lands here first.
const WIRE_SCHEMA_GOLDEN: &str = "046ff264f692eb93d76f15267f31b986a139f9b6829b877e27211e863e76ebc8";

/// The schema hash reads names only, so a field added under names that
/// stay leaves it where it was. This digest reads the whole shape with the
/// prose taken out: it turns red on every shape change, and the person who
/// sees it decides whether this change is the one that raises `WIRE_V`.
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
         WIRE_V in this commit (wire-SPEC 12.1); then set WIRE_SHAPE_GOLDEN to the digest above"
    );
}

/// Removes every doc string the schema carries, so an edited comment
/// leaves the digest alone. Only string values go: a field that happens to
/// be called `description` is an object under `properties` and stays.
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

/// The digest of `wire_schema()` with its prose removed.
#[cfg(feature = "schema")]
const WIRE_SHAPE_GOLDEN: &str = "a309e857a4f4a163ce27e7dd89edbaf1ce5a555c3413fadfa2558768dbdd3de9";

// -------------------------------------------------------------- binding face

#[cfg(feature = "server")]
#[test]
fn the_binding_face_has_exactly_one_refusing_cell() {
    // Constitution 8.3: loopback by default; exposed requires a pairing token;
    // no token configured means refuse to *start*, not refuse to connect. The
    // credential the served face demands comes back inside the verdict, so the
    // shell that carries it cannot demand something else than the face it is.
    let secret = kernel::B3Hash::digest(b"pair-me-during-binding");
    assert!(matches!(
        decide_bind(&loopback(), None),
        BindVerdict::Serve(BindFace::Loopback { token: None })
    ));
    assert!(matches!(
        decide_bind(&loopback(), Some(secret)),
        BindVerdict::Serve(BindFace::Loopback { token: Some(_) })
    ));
    let BindVerdict::Serve(exposed_face) = decide_bind(&exposed(), Some(secret)) else {
        panic!("an exposed bind with a token is served");
    };
    assert_eq!(exposed_face.token_digest(), Some(&secret));

    let BindVerdict::Refuse(err) = decide_bind(&exposed(), None) else {
        panic!("an exposed bind with no token must refuse to start");
    };
    assert_eq!(*err.code(), AxCode::ConfigInvalid);
    assert!(
        !err.recovery().is_empty(),
        "a refusal names an executable alternative"
    );
    // The one pair of cells that differ: a token is demanded beyond this
    // machine and optional on it.
    assert_ne!(
        BindFace::Loopback { token: None }.token_digest(),
        BindFace::Exposed { token: secret }.token_digest()
    );
}

// ------------------------------------------------------------------ handshake

#[cfg(feature = "server")]
#[test]
fn a_mismatched_schema_hash_is_rejected_before_anything_else() {
    let good = Hello {
        wire_v: WIRE_V,
        schema: schema_hash(),
        token: None,
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

/// The face of a city nobody configured a token for: reachable from this
/// machine only, demanding nothing.
#[cfg(feature = "server")]
fn loose() -> BindFace {
    BindFace::Loopback { token: None }
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
    let paired = BindFace::Exposed { token: secret };

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
    // The same hello, on the face of a city that configured no token, is
    // admitted: what differs is the face, and nothing else decides.
    assert!(matches!(
        decide_handshake(&absent, &expected, &loose()),
        HandshakeVerdict::Accept
    ));
}

// --------------------------------------------------- the two unspellable shapes

#[test]
fn put_secret_has_no_byte_form_in_either_direction() {
    // A13 continues onto the wire. The outbound half is a *compile* error -
    // `Command<Sealed<String>>` has no `Serialize` because `Sealed` has none,
    // so the trybuild case in tests/compile_fail is the real proof and this
    // test cannot even spell the attempt. The inbound half is checked here:
    // bytes that name the variant are refused, and the refusal stays silent
    // about what it was protecting.
    let json = r#"{"put_secret":{"realm":"anthropic","name":"api","value":"sk-not-a-real-key"}}"#;
    let decoded: Result<wire::WireCommand, _> = serde_json::from_str(json);
    let err = decoded.expect_err("PutSecret has no wire form");
    assert!(
        !err.to_string().contains("sk-not-a-real-key"),
        "the refusal must not echo the very bytes it is protecting"
    );

    // The in-process shape still exists - that is the whole point: the host
    // may enrol a credential, a socket may not.
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
    // Constitution 8.3: "double-clicking twice does not open two Runs."
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

// ----------------------------------------------------------- retired frames

/// The two frames that left the wire rather than staying on it answered
/// with `not_built` for ever (wire-SPEC.md section 8-44,
/// kernel-SPEC.md section 12.2). Their bytes still spell what they
/// spelled, and that is no longer a command: what a client gets for them
/// is the grammar's own unknown-variant refusal, not a verb it may offer
/// a person.
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

// ------------------------------------------------------------------- fixtures

fn sample_of_every_command() -> Vec<Command> {
    let addr = Address::parse("acme/floor1").unwrap();
    let run = kernel::RunId::from_bytes([7u8; 16]);
    let idem = kernel::IdemKey::derive(&run, Seq::new(1), b"sample");
    vec![
        Command::Wake {
            source: "github".to_owned(),
            subject: "pull request opened".to_owned(),
            body: "someone would like a change reviewed".to_owned(),
            idem,
        },
        Command::Dispatch {
            addr: addr.clone(),
            task: "ship it".to_owned(),
            goal: "the tests pass".to_owned(),
            policy: wire::RunPolicy::of(Mode::Work),
            idem,
            session: Some(kernel::SessionName::parse("ship it").unwrap()),
            effort: Some(kernel::Effort::High),
            model: None,
        },
        Command::PutDocument {
            which: wire::GovernedDocument::Mayor,
            body: "# who the Mayor is
"
            .to_owned(),
            idem,
        },
        Command::PutSpine {
            building: Address::parse("lab").unwrap(),
            which: wire::SpineDocument::Memo,
            base: String::new(),
            body: "# Memo — lab
"
            .to_owned(),
            idem,
        },
        Command::ConfigureBuilding {
            addr: Address::parse("lab").unwrap(),
            sandbox: Some(kernel::SandboxLimits::default()),
            mcp: Some(Vec::new()),
            desktop: Some(
                "[[window]]
title = \"a window\"
"
                .to_owned(),
            ),
            context_second_threshold: None,
            idem,
        },
        Command::ProbeEndpoint {
            name: ProviderName::parse("house").unwrap(),
            base_url: "https://api.example.test/v1".to_owned(),
            dialect: kernel::DialectKind::OpenAi,
            secret: Some("secret:house/key".to_owned()),
            auth_header: None,
            tuning: wire::EndpointTuning::default(),
            idem,
        },
        Command::AttachEndpoint {
            name: ProviderName::parse("house").unwrap(),
            base_url: "https://api.example.test/v1".to_owned(),
            dialect: kernel::DialectKind::OpenAi,
            secret: Some("secret:house/key".to_owned()),
            auth_header: None,
            admit: vec!["gpt-x".to_owned()],
            tuning: wire::EndpointTuning {
                label: Some("House".to_owned()),
                timeout_ms: Some(60_000),
                request_max_retries: Some(4),
                stream_idle_timeout_ms: Some(300_000),
                headers: vec![wire::HeaderPair {
                    name: "x-tenant".to_owned(),
                    value: "secret:house/tenant".to_owned(),
                }],
                overrides: vec![wire::BodyOverride {
                    pointer: "/reasoning/effort".to_owned(),
                    value: "high".to_owned(),
                }],
                proxying: Some(kernel::Proxying::Always),
            },
            idem,
        },
        Command::SelectModel {
            endpoint: ProviderName::parse("house").unwrap(),
            model: "m-large".to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: wire::Window::new(128_000),
            max_output_tokens: kernel::Ceiling::new(8_192),
            idem,
        },
        Command::OpenSession {
            addr: addr.clone(),
            carry: wire::Carry::Handoff,
            from: Some(kernel::Origin {
                run,
                at_seq: Seq::new(3),
            }),
            idem,
        },
        Command::CreateBuilding {
            addr: addr.clone(),
            template: wire::TemplateName::parse("workshop").unwrap(),
            idem,
        },
        Command::RemoveBuilding {
            addr: addr.clone(),
            idem,
        },
        Command::PutSecret {
            realm: "anthropic".to_owned(),
            name: "api".to_owned(),
            value: Sealed::new(Box::new("sk-not-a-real-key".to_owned())),
        },
        Command::Steer {
            run,
            text: "try the other branch".to_owned(),
            idem,
        },
        Command::Cancel { run, idem },
        Command::Halt {
            scope: wire::HaltScope::City,
            idem,
        },
        Command::Release {
            scope: wire::HaltScope::City,
            idem,
        },
        Command::BatchByBuilding {
            addr: addr.clone(),
            idem,
        },
        Command::Approve {
            item: kernel::ApprovalId::new("ap-1").unwrap(),
            verdict: kernel::Ruling::Allow,
            idem,
        },
        Command::HandOff {
            item: kernel::ApprovalId::new("ap-1").unwrap(),
            to: kernel::ResidentId::new("clerk").unwrap(),
            idem,
        },
        Command::PutPreferences {
            patch: wire::PreferencePatch::Lang(wire::Lang::Zh),
            idem,
        },
        Command::PutShelved {
            shelf: wire::Shelf::Library,
            name: "reviewing/first-pass.md".to_owned(),
            text: "# read the SPEC first\n".to_owned(),
            idem,
        },
        Command::SetAutonomy {
            scope: wire::HaltScope::City,
            autonomy: kernel::Autonomy::Owner,
            idem,
        },
        Command::Pursue {
            addr,
            step: wire::PursuitStep::Set {
                goal: "raise the east wing".to_owned(),
            },
            idem,
        },
        Command::Auth {
            token: "pair-me-0123456789".to_owned(),
        },
        Command::Reveal {
            at: Address::parse("hall/JOB.md").unwrap(),
            idem,
        },
        Command::RestoreDiscard {
            restoration: kernel::Restoration::Rebuildable {
                reason: "regenerated by the build".to_owned(),
            },
            idem,
        },
        Command::DoctorInstall {
            item: "cargo-nextest".to_owned(),
            idem,
        },
        Command::DoctorRefresh { idem },
        Command::ConnectToolkit {
            toolkit: wire::ToolkitSlug::parse("github").unwrap(),
            idem,
        },
    ]
}

/// A branch names the run and the line; a session that begins without
/// one sends `null`, which is how every optional field of a Command is
/// spelled on this wire (`Dispatch.session` and `effort` are the other
/// two). The ledger's records spell absence by leaving a key out, and
/// that difference is deliberate: a record is read by this build, and a
/// frame is read by whatever is on the other end of a socket.
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
    // Queries are side-effect free by construction, so there is nothing to
    // deduplicate; if one ever needs a key it has stopped being a Query.
    for name in QUERY_NAMES {
        assert!(!name.is_empty());
    }
    assert!(matches!(Query::ApprovalQueue, Query::ApprovalQueue));
}

/// Narrowing history to one session survives the wire, and stays a
/// different frame from the unfiltered slice.
///
/// Asked as one question, four sessions would divide one bounded slice
/// between them, and a session older than that slice would open blank.
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

/// The range a slow session lost travels with both of its ends, and the
/// question that fills it is a frame of its own.
///
/// A reader told only how many records went by cannot ask for them: the
/// count names no endpoint. It is deliberately not `History`, because an
/// answer to that carries a cursor for a walk backwards and a gap has a
/// near end as well as a far one.
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

/// Nobody can price a piece of work before it runs, so the frame that
/// starts one carries no ceiling. The one brake is `Halt`,
/// which shuts a scope and stops what that scope already started.
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

/// The three documents that govern a city are written by one frame, and
/// what was decided on the person's behalf is one question.
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
        body: "# how I like this city run\n".to_owned(),
        idem: kernel::IdemKey::derive(&kernel::RunId::from_bytes([9u8; 16]), Seq::new(1), b"prefs"),
    };
    assert_eq!(frame.name(), "PutDocument");
    let text = serde_json::to_string(&frame).unwrap();
    let back: wire::WireCommand = serde_json::from_str(&text).unwrap();
    assert_eq!(back, frame);
    assert_eq!(Query::Governance.name(), "Governance");
}

/// A hunk is its own request: one file, both ends named, and a line that
/// matched a credential shape reported rather than echoed.
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
    // Counts and patch text are two questions with two costs, so they
    // are two frames.
    assert_ne!(
        bytes,
        serde_json::to_vec(&Query::Changes {
            base: kernel::GitOid::from_bytes([1u8; 20]),
            head: Some(kernel::GitOid::from_bytes([2u8; 20])),
        })
        .unwrap()
    );
}
