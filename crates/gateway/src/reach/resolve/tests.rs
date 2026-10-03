// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One round per preset host and face (`crates/gateway/spec/Reach/Resolve.lean` §8-32):
//! the request the production path writes for that host, heard by a
//! stand-in, against what the host's vendor documents.

use kernel::{AxCode, DialectKind, SecretRef};

use super::{Heard, StandIn};
use crate::endpoint::{AuthSpec, Transport};
use crate::provider::preset::{CeilingField, Face, HostPreset, PRESETS};
use crate::router::{AttachedEndpoint, Chosen, DialectHint, EndpointTuning, normalise_entered};

/// The key the test redemption hands out for every reference.
const KEY: &str = "sk-test-0123456789";

/// What one request looked like, in the terms a vendor documents it.
#[derive(Debug, PartialEq)]
struct Shape {
    /// Which round this is: the host and the face.
    round: String,
    path: String,
    host: Option<String>,
    /// The header the credential travelled in, and its value.
    credential: Option<(String, String)>,
    /// Every conversation header any preset host asks for that arrived.
    sessions: Vec<String>,
    dialect: DialectKind,
    /// For the chat face: the field the output ceiling was written in.
    ceiling: Option<&'static str>,
}

#[test]
fn every_preset_host_and_face_is_heard_in_the_shape_its_vendor_documents() {
    let rounds: Vec<(&HostPreset, &Face)> = PRESETS
        .iter()
        .flat_map(|row| row.faces.iter().map(move |face| (row, face)))
        .collect();
    let heard: Vec<Shape> = rounds.iter().map(|(row, face)| heard(row, face)).collect();
    let owed: Vec<Shape> = rounds.iter().map(|(row, face)| owed(row, face)).collect();
    assert_eq!(heard, owed);
}

fn round(row: &HostPreset, face: &Face) -> String {
    format!("{} {:?}", row.host, face.dialect)
}

/// What the vendor documents for one face of one host.
fn owed(row: &HostPreset, face: &Face) -> Shape {
    let (suffix, credential) = match face.dialect {
        DialectKind::Anthropic => ("messages", ("x-api-key", KEY.to_owned())),
        DialectKind::OpenAi => (
            "chat/completions",
            ("authorization", format!("Bearer {KEY}")),
        ),
        DialectKind::OpenAiResponses => ("responses", ("authorization", format!("Bearer {KEY}"))),
    };
    Shape {
        round: round(row, face),
        path: format!("{}/{suffix}", face.path.trim_end_matches('/')),
        host: Some(row.host.to_owned()),
        credential: Some((credential.0.to_owned(), credential.1)),
        sessions: row.session_header.map(str::to_owned).into_iter().collect(),
        dialect: face.dialect,
        ceiling: match face.dialect {
            DialectKind::OpenAi => Some(match row.chat.ceiling {
                CeilingField::MaxTokens => "max_tokens",
                CeilingField::MaxCompletionTokens => "max_completion_tokens",
            }),
            DialectKind::Anthropic | DialectKind::OpenAiResponses => None,
        },
    }
}

/// What a stand-in heard when the production path called one face of
/// one host: the base URL attaching stores, the connection it resolves
/// to, the credential header its face takes, and `adapter_for`.
fn heard(row: &HostPreset, face: &Face) -> Shape {
    let stand_in = StandIn::listening(row.host);
    let hint = DialectHint::of(face.dialect);
    let stored = normalise_entered(row.host, hint).unwrap();
    let kind = crate::provider::registry::resolve(hint).unwrap();
    let endpoint = AttachedEndpoint {
        name: row.host.to_owned(),
        base_url: stored.base_url,
        dialect: kind.wire(),
        connection_kind: kind,
        auth: AuthSpec::for_dialect(
            kind.wire(),
            SecretRef::parse("secret:stand-in/key").unwrap(),
            None,
        ),
        models: Vec::new(),
        probed: false,
        tuning: EndpointTuning::default(),
    };
    let entry = crate::market::MarketSnapshot::builtin()
        .unwrap()
        .lookup("local")
        .unwrap()
        .clone();
    let transport = Transport::detoured(stand_in.toward());
    let chosen = Chosen {
        endpoint: &endpoint,
        entry: &entry,
        transport: &transport,
    };
    let mut model = crate::adapter_for(
        &chosen,
        crate::endpoint::redemption::redemption(),
        Vec::new(),
        crate::endpoint::fakes::monotonic,
    )
    .unwrap();
    // A system prompt, so the three bodies differ where `shape_of`
    // reads them.
    let mut request = crate::endpoint::fakes::request();
    request.chat.system.push(kernel::SystemBlock {
        text: "Answer briefly.".to_owned(),
        cache: false,
    });
    let refused = model.call(&request).unwrap_err();
    assert_eq!(*refused.code(), AxCode::Provider, "{refused}");
    shape_of(round(row, face), &stand_in.heard())
}

/// One request taken apart into the terms `owed` speaks in.
fn shape_of(round: String, heard: &Heard) -> Shape {
    let header = |name: &str| heard.headers.get(name).cloned();
    let credential = ["x-api-key", "authorization"]
        .into_iter()
        .find_map(|name| header(name).map(|value| (name.to_owned(), value)));
    let mut sessions: Vec<String> = PRESETS
        .iter()
        .filter_map(|row| row.session_header)
        .filter(|name| heard.headers.contains_key(*name))
        .map(str::to_owned)
        .collect();
    sessions.sort_unstable();
    sessions.dedup();
    // The three bodies differ at the top: Responses sends `input`,
    // Anthropic sends `system` beside its messages, and the chat face
    // sends its system prompt as one of the messages.
    let dialect = if heard.body.get("input").is_some() {
        DialectKind::OpenAiResponses
    } else if heard.body.get("system").is_some() {
        DialectKind::Anthropic
    } else {
        DialectKind::OpenAi
    };
    let ceiling = match dialect {
        DialectKind::OpenAi => ["max_tokens", "max_completion_tokens"]
            .into_iter()
            .find(|field| heard.body.get(*field).is_some()),
        DialectKind::Anthropic | DialectKind::OpenAiResponses => None,
    };
    Shape {
        round,
        path: heard.path.clone(),
        host: header("host"),
        credential,
        sessions,
        dialect,
        ceiling,
    }
}
