// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A dispatch driven against a model the worker was handed
//! (accounting-SPEC.md section 2).
//!
//! The endpoint the city is pointed at refuses every connection, so the
//! scripted answer can reach the history only through the factory the
//! worker received. A worker that still built its own adapter out of the
//! endpoint book would call the dead port and never meet the script.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use kernel::{
    Address, AxError, ContentBlock, EventKind, EventRecord, IdemKey, Model, ModelRequest,
    ModelReturn, RunId, Seq,
};
use sprawling::assembly;

const LAB: &str = "lab";
const MODEL: &str = "scripted";
const ANSWER: &str = "the answer the script wrote";

/// Answers every call with the same text and no tool calls, so the run
/// ends on its first turn.
struct Script;

impl Model for Script {
    fn call(&mut self, _req: &ModelRequest) -> Result<ModelReturn, AxError> {
        Ok(ModelReturn::bare(
            kernel::model::message_payload(&[ContentBlock::Text {
                text: ANSWER.to_owned(),
            }])?,
            Vec::new(),
        ))
    }
}

struct Scripted;

impl accounting::ModelFactory for Scripted {
    fn build(
        &self,
        _chosen: &gateway::Chosen<'_>,
        _redemption: gateway::Redemption,
    ) -> Result<Box<dyn Model + Send>, AxError> {
        Ok(Box::new(Script))
    }
}

/// A loopback address nothing listens on: bound once for a free port,
/// then released.
fn refusing_url() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    format!("http://127.0.0.1:{port}/v1")
}

fn idem(what: &[u8]) -> IdemKey {
    IdemKey::derive(&RunId::CITY, Seq::FIRST, what)
}

#[test]
fn a_dispatch_reaches_the_model_the_worker_was_handed() {
    let dir = tempfile::tempdir().unwrap();
    let raised = assembly::init_city(dir.path()).unwrap();
    let mut worker = assembly::RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap()
    .with_models(Box::new(Scripted));
    let endpoint = channels::ProviderName::parse("dead").unwrap();
    worker
        .handle(channels::Command::AttachEndpoint {
            name: endpoint.clone(),
            base_url: refusing_url(),
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            admit: vec![MODEL.to_owned()],
            // One try, briefly: a worker that still calls the dead port
            // should fail this test at once rather than retry into it.
            tuning: channels::EndpointTuning {
                timeout_ms: Some(2_000),
                request_max_retries: Some(0),
                ..channels::EndpointTuning::default()
            },
            idem: idem(b"attach"),
        })
        .unwrap();
    worker
        .handle(channels::Command::SelectModel {
            endpoint,
            model: MODEL.to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: kernel::Window::new(32_768),
            max_output_tokens: kernel::Ceiling::new(1_024),
            idem: idem(b"select"),
        })
        .unwrap();
    worker
        .handle(channels::Command::CreateBuilding {
            addr: Address::parse(LAB).unwrap(),
            template: channels::TemplateName::parse("minimal").unwrap(),
            idem: idem(b"create"),
        })
        .unwrap();
    // Judged on the history rather than on this result: a run that met
    // the dead port is a history with a failure in it, not necessarily a
    // refused command.
    let dispatched = worker.handle(channels::Command::Dispatch {
        addr: Address::parse(LAB).unwrap(),
        task: "Answer.".to_owned(),
        goal: "one answer from the model this worker was handed".to_owned(),
        mode: kernel::Mode::PlanGoal,
        idem: idem(b"dispatch"),
        // Named, so the room is opened rather than named by the digest
        // model: this test is about the run's model and nothing else.
        session: Some(kernel::SessionName::parse("s1").unwrap()),
        effort: None,
    });

    let verified = runtime::replay::verify_ledger_dir(&raised.ledger_dir).unwrap();
    let scripted = verified.raw_lines().iter().any(|line| {
        EventRecord::parse_line(line).unwrap().kind() == EventKind::ModelReturned
            && String::from_utf8_lossy(line).contains(ANSWER)
    });
    assert!(
        scripted,
        "the run never heard the scripted model (the dispatch answered {dispatched:?})"
    );
}
