// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A dispatch driven against the models the worker was handed
//! (`crates/accounting/Spec.lean` section 2).
//!
//! The endpoint the city is pointed at refuses every connection, so a
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

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use kernel::{
    Address, AxCode, AxError, ContentBlock, EventKind, EventRecord, IdemKey, Model, ModelRequest,
    ModelReturn, RunId, Seq,
};
// The city is formed with an in-memory vault, because these tests are
// about the worker and must not write to the credential service of the
// machine that runs them.
use accounting::worker::genesis::{Adopt, form};
use sprawling::assembly;

const LAB: &str = "lab";
const MAIN: &str = "scripted";
const DIGEST: &str = "namer";
const ANSWER: &str = "the answer the script wrote";
/// The room the rule names from the task `Answer.`.
const ROOM: &str = "answer";

/// Answers every call with the same text and no tool calls, so a run
/// ends on its first turn.
struct Script(&'static str);

impl Model for Script {
    fn call(&mut self, _req: &ModelRequest) -> Result<ModelReturn, AxError> {
        Ok(ModelReturn::bare(
            kernel::model::message_payload(&[ContentBlock::Text {
                text: self.0.to_owned(),
            }])?,
            Vec::new(),
        ))
    }
}

/// Hands out a `Script` for every model, and keeps the id of each model
/// it was asked for.
struct Scripted {
    says: &'static str,
    asked: Arc<Mutex<Vec<String>>>,
}

impl accounting::ModelFactory for Scripted {
    fn build(
        &self,
        chosen: &gateway::Chosen<'_>,
        _redemption: gateway::Redemption,
    ) -> Result<Box<dyn Model + Send>, AxError> {
        self.asked.lock().unwrap().push(chosen.entry.id.clone());
        Ok(Box::new(Script(self.says)))
    }
}

fn scripted(says: &'static str) -> (Scripted, Arc<Mutex<Vec<String>>>) {
    let asked = Arc::new(Mutex::new(Vec::new()));
    let factory = Scripted {
        says,
        asked: Arc::clone(&asked),
    };
    (factory, asked)
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

/// A city whose main and digest models both sit on `base_url`, whose
/// one building is raised from `template`, and whose worker reaches
/// every model through `factory`; with the directory its ledger is in.
fn city_on(
    dir: &Path,
    base_url: String,
    template: &str,
    factory: Scripted,
) -> (accounting::worker::RunWorker, PathBuf) {
    let raised = form(
        dir,
        Adopt::Nothing,
        assembly::hands(gateway::Custodian::in_memory()),
    )
    .unwrap();
    let mut worker = accounting::worker::RunWorker::new(
        dir,
        runtime::diagnostics::Diagnostics::off(),
        assembly::hands(gateway::Custodian::in_memory()),
    )
    .unwrap()
    .with_models(Box::new(factory));
    let endpoint = wire::ProviderName::parse("dead").unwrap();
    worker
        .handle(wire::Command::AttachEndpoint {
            name: endpoint.clone(),
            base_url,
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            admit: vec![MAIN.to_owned(), DIGEST.to_owned()],
            // One try, briefly: a worker that still calls the dead port
            // should fail its test at once rather than retry into it.
            tuning: wire::EndpointTuning {
                timeout_ms: Some(2_000),
                request_max_retries: Some(0),
                ..wire::EndpointTuning::default()
            },
            idem: idem(b"attach"),
        })
        .unwrap();
    for (tag, model) in [
        (kernel::ModelTag::Main, MAIN),
        (kernel::ModelTag::Digest, DIGEST),
    ] {
        worker
            .handle(wire::Command::SelectModel {
                endpoint: endpoint.clone(),
                model: model.to_owned(),
                tag,
                context_tokens: kernel::Window::new(32_768),
                max_output_tokens: kernel::Ceiling::new(1_024),
                input: None,
                idem: idem(model.as_bytes()),
            })
            .unwrap();
    }
    worker
        .handle(wire::Command::CreateBuilding {
            addr: Address::parse(LAB).unwrap(),
            template: wire::TemplateName::parse(template).unwrap(),
            idem: idem(b"create"),
        })
        .unwrap();
    (worker, raised.ledger_dir)
}

fn dispatch(
    worker: &mut accounting::worker::RunWorker,
    session: Option<kernel::SessionName>,
) -> Result<(), AxError> {
    worker.handle(wire::Command::Dispatch {
        addr: Address::parse(LAB).unwrap(),
        task: "Answer.".to_owned(),
        goal: "one answer from the model this worker was handed".to_owned(),
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        idem: idem(b"dispatch"),
        session,
        effort: None,
        model: None,
    })
}

/// Whether the history holds a line of `kind` that carries `text`.
fn history_says(ledger: &Path, kind: EventKind, text: &str) -> bool {
    runtime::replay::verify_ledger_dir(ledger)
        .unwrap()
        .raw_lines()
        .iter()
        .any(|line| {
            EventRecord::parse_line(line).unwrap().kind() == kind
                && String::from_utf8_lossy(line).contains(text)
        })
}

#[test]
fn a_dispatch_reaches_the_model_the_worker_was_handed() {
    let dir = tempfile::tempdir().unwrap();
    let (factory, _) = scripted(ANSWER);
    let (mut worker, ledger) = city_on(dir.path(), refusing_url(), "minimal", factory);
    // Named, so the room is opened rather than named from the task:
    // this test is about the run's model and nothing else.
    // Judged on the history rather than on this result: a run that met
    // the dead port is a history with a failure in it, not necessarily a
    // refused command.
    let dispatched = dispatch(&mut worker, Some(kernel::SessionName::parse("s1").unwrap()));

    assert!(
        history_says(&ledger, EventKind::ModelReturned, ANSWER),
        "the run never heard the scripted model (the dispatch answered {dispatched:?})"
    );
}

#[test]
fn an_unnamed_dispatch_asks_the_factory_for_the_run_model_alone() {
    let dir = tempfile::tempdir().unwrap();
    let (factory, asked) = scripted(ANSWER);
    let (mut worker, ledger) = city_on(dir.path(), refusing_url(), "minimal", factory);
    // No session, so the room is named from the task's own words by
    // rule (`crates/sprawling/Spec.lean` §8-86): the factory is asked for the run's
    // model and never for a digest model to name the room with.
    let dispatched = dispatch(&mut worker, None);

    assert_eq!(
        (
            history_says(&ledger, EventKind::RunStarted, &format!("{LAB}/{ROOM}")),
            asked.lock().unwrap().clone(),
        ),
        (true, vec![MAIN.to_owned()]),
        "the dispatch answered {dispatched:?}"
    );
}

#[test]
fn a_confidential_building_refuses_before_the_factory_is_asked() {
    let dir = tempfile::tempdir().unwrap();
    let (factory, asked) = scripted(ANSWER);
    // A documentation address (RFC 5737): not on this machine, and
    // never dialled, because the refusal comes before any adapter.
    let (mut worker, _) = city_on(
        dir.path(),
        "http://192.0.2.1/v1".to_owned(),
        "confidential",
        factory,
    );
    let refused = dispatch(&mut worker, Some(kernel::SessionName::parse("s1").unwrap()))
        .map_err(|err| *err.code());

    assert_eq!(
        (refused, asked.lock().unwrap().clone()),
        (Err(AxCode::GateDenied), Vec::<String>::new())
    );
}
