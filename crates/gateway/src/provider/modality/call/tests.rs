// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One call to a vector face, against loopback servers that answer the
//! two shapes and refuse the two ways.

use super::*;
use crate::endpoint::AuthSpec;
use crate::endpoint::fakes;
use crate::endpoint::redemption::redemption;
use crate::provider::registry::{ConnectionKind, Family};
use crate::router::EndpointTuning;
use kernel::event::Payload;

/// One endpoint attached at the base a fake provider answered under.
///
/// The fake answers on a chat path; a registration states the base
/// every face hangs off, which is the part before it.
fn attached(url: &str, kind: ConnectionKind) -> AttachedEndpoint {
    AttachedEndpoint {
        name: "local".to_owned(),
        base_url: url.trim_end_matches("/messages").to_owned(),
        dialect: kind.wire(),
        connection_kind: kind,
        auth: AuthSpec::None,
        models: Vec::new(),
        probed: false,
        tuning: EndpointTuning::default(),
    }
}

fn vectors(body: &str, status: u16) -> (AttachedEndpoint, std::thread::JoinHandle<Vec<String>>) {
    let (url, server) = fakes::fake_provider(vec![(status, body.to_owned())], false);
    (attached(&url, ConnectionKind::OpenAiCompat), server)
}

/// One embedding call leaves through the endpoint's own transport,
/// comes back read, and hands the ledger the line that records it.
#[test]
fn an_embedding_call_reads_the_answer_and_records_the_line() {
    let answer = serde_json::json!({
        "model": "bge-m3",
        "data": [
            { "index": 1, "embedding": [0.5, 1.5] },
            { "index": 0, "embedding": [1.0, 2.0] },
        ],
        "usage": { "prompt_tokens": 77 },
    })
    .to_string();
    let (endpoint, server) = vectors(&answer, 200);
    let asked = EmbeddingRequest::new(
        "bge-m3".to_owned(),
        vec!["one".to_owned(), "two".to_owned()],
    )
    .unwrap()
    .with_dimensions(2);
    let call = Vectors::of(&endpoint, "bge-m3".to_owned()).unwrap();

    let (read, record) = call.embed(&asked, redemption()).unwrap();
    assert_eq!(
        call.url(),
        format!("{}/embeddings", endpoint.base_url),
        "the face hangs off the base the person registered"
    );
    assert_eq!(
        read.vectors(),
        vec![vec![1.0, 2.0], vec![0.5, 1.5]],
        "the index pairs a vector with its text, not the arrival order"
    );
    assert_eq!(
        record,
        EmbeddingCalled {
            model: "bge-m3".to_owned(),
            inputs: 2,
            vectors: 2,
            dimensions: Some(2),
            prompt_tokens: Some(Tokens::new(77)),
        }
    );
    let seen = server.join().unwrap();
    assert!(
        seen[0].contains("\"encoding_format\":\"float\""),
        "{}",
        seen[0]
    );
    assert!(
        seen[0].starts_with("POST /v1/embeddings "),
        "the path is the one this connection serves: {}",
        seen[0]
    );
    assert!(
        seen[0].contains("\"model\":\"bge-m3\""),
        "the provider's own id for the model travels, not the conversation's: {}",
        seen[0]
    );
}

/// The line travels as the payload a ledger row carries, and reads
/// back as the record that was written.
#[test]
fn the_record_is_a_ledger_payload() {
    let answer = serde_json::json!({
        "data": [ { "index": 0, "embedding": [1.0] } ],
    })
    .to_string();
    let (endpoint, server) = vectors(&answer, 200);
    let asked = EmbeddingRequest::new("bge-m3".to_owned(), vec!["one".to_owned()]).unwrap();
    let call = Vectors::of(&endpoint, "bge-m3".to_owned()).unwrap();
    let (_, record) = call.embed(&asked, redemption()).unwrap();
    drop(server);

    let payload = Payload::of(&record).unwrap();
    assert_eq!(
        payload.as_map().keys().collect::<Vec<_>>(),
        vec!["inputs", "model", "vectors"],
        "what was not stated is absent rather than zero"
    );
    assert_eq!(payload.read::<EmbeddingCalled>().unwrap(), record);
}

/// A rerank face takes the same road, and its own answer shape.
#[test]
fn a_rerank_call_reads_the_answer_and_records_the_line() {
    let answer =
        serde_json::json!([{ "index": 1, "score": 9.5 }, { "index": 0, "score": 1.0 }]).to_string();
    let (endpoint, server) = vectors(&answer, 200);
    let asked = RerankRequest::new(
        "which one?".to_owned(),
        vec!["first".to_owned(), "second".to_owned()],
    )
    .unwrap();
    let call = Ranks::of(&endpoint, "bge-reranker".to_owned()).unwrap();

    let (read, record) = call.rank(&asked, redemption()).unwrap();
    assert_eq!(read.best().unwrap().passage, 1);
    assert_eq!(
        record,
        RerankCalled {
            model: "bge-reranker".to_owned(),
            passages: 2,
            ranks: 2,
            prompt_tokens: None,
        }
    );
    let seen = server.join().unwrap();
    assert!(seen[0].starts_with("POST /v1/rerank "), "{}", seen[0]);
}

/// A connection that serves no such face is refused before a byte
/// leaves, and the refusal says which connection it was.
#[test]
fn a_subscription_is_refused_before_anything_is_sent() {
    for family in Family::ALL {
        let endpoint = attached("http://127.0.0.1:1", ConnectionKind::Harness(family));
        for refusal in [
            Vectors::of(&endpoint, "bge-m3".to_owned()).err(),
            Ranks::of(&endpoint, "bge-reranker".to_owned()).err(),
        ] {
            let refused = refusal.expect("a subscription sells a conversation, not a vector");
            assert_eq!(*refused.code(), AxCode::ConfigInvalid);
            assert!(
                refused.subject().contains(family.as_str()),
                "{}",
                refused.subject()
            );
            assert!(
                refused.recovery().contains("embedding") || refused.recovery().contains("rerank")
            );
        }
    }
    let messages = attached("http://127.0.0.1:1", ConnectionKind::AnthropicNative);
    assert!(Ranks::of(&messages, "x".to_owned()).is_err());
}

/// The responses face serves embeddings and no rerank, which is the
/// table's answer and not this module's.
#[test]
fn the_responses_face_serves_one_of_the_two() {
    let endpoint = attached("http://127.0.0.1:1", ConnectionKind::Responses);
    assert!(Vectors::of(&endpoint, "bge-m3".to_owned()).is_ok());
    assert!(Ranks::of(&endpoint, "bge-reranker".to_owned()).is_err());
}

/// A provider that refuses the call is an error, never a partial
/// answer.
#[test]
fn a_non_success_status_is_e_provider() {
    let (endpoint, server) = vectors("{}", 404);
    let asked = EmbeddingRequest::new("bge-m3".to_owned(), vec!["one".to_owned()]).unwrap();
    let call = Vectors::of(&endpoint, "bge-m3".to_owned()).unwrap();
    let err = call.embed(&asked, redemption()).unwrap_err();
    assert_eq!(*err.code(), AxCode::Provider);
    drop(server);
}
