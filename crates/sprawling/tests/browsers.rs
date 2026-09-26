// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A run offered the browser the worker was handed
//! (sprawling-SPEC.md section 8-45-2).
//!
//! The building's rules ask for a browser, and the worker was handed a
//! source of browser tools that answers with one scripted tool. A worker
//! that still built its own would offer the model the host's browser
//! tool under its own name, and never the scripted one.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::sync::{Arc, Mutex};

use kernel::{
    Address, AxError, ContentBlock, IdemKey, Model, ModelRequest, ModelReturn, RunId, Seq,
};
use sprawling::assembly;

const LAB: &str = "lab";
const MODEL: &str = "scripted";
const HANDED: &str = "scripted_browser";

/// Keeps the name of every tool it is offered, and ends the run on its
/// first turn.
struct Listening(Arc<Mutex<Vec<String>>>);

impl Model for Listening {
    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
        self.0.lock().unwrap().extend(
            req.chat
                .tools
                .iter()
                .map(|tool| tool.name.as_str().to_owned()),
        );
        Ok(ModelReturn::bare(
            kernel::model::message_payload(&[ContentBlock::Text {
                text: "done".to_owned(),
            }])?,
            Vec::new(),
        ))
    }
}

struct Listeners(Arc<Mutex<Vec<String>>>);

impl accounting::ModelFactory for Listeners {
    fn build(
        &self,
        _chosen: &gateway::Chosen<'_>,
        _redemption: gateway::Redemption,
    ) -> Result<Box<dyn Model + Send>, AxError> {
        Ok(Box::new(Listening(Arc::clone(&self.0))))
    }
}

/// A browser that opens nothing: it is only ever listed.
struct Scripted(kernel::ToolMeta);

impl kernel::Tool for Scripted {
    fn meta(&self) -> &kernel::ToolMeta {
        &self.0
    }

    fn invoke(&self, _call: &kernel::ToolCall) -> Result<kernel::ToolOutcome, AxError> {
        Ok(kernel::ToolOutcome {
            result: kernel::Payload::new(serde_json::Map::new())?,
            attachments: Vec::new(),
        })
    }
}

/// Answers every building that asks for a browser with the scripted one.
fn scripted(
    _city_root: &std::path::Path,
    _origin: &memory::BlockOrigin,
    rules: &city::BuildingRules,
) -> Result<Vec<Box<dyn kernel::Tool>>, AxError> {
    let meta = kernel::ToolMeta {
        name: kernel::ToolName::parse(HANDED)?,
        disclosure: "A browser that opens nothing; it is only ever listed.".to_owned(),
        params: kernel::Payload::new(serde_json::Map::new())?,
        effect: kernel::Effect::Read,
        cost_tier: kernel::CostTier::Light,
        timeout: None,
        render: kernel::RenderIntent::Generic,
        temporal: kernel::Temporal::Timeless,
    };
    Ok(if rules.browser() {
        vec![Box::new(Scripted(meta))]
    } else {
        Vec::new()
    })
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
fn a_run_is_offered_the_browser_the_worker_was_handed() {
    let dir = tempfile::tempdir().unwrap();
    assembly::init_city(dir.path()).unwrap();
    let offered = Arc::new(Mutex::new(Vec::new()));
    let mut worker = assembly::RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap()
    .with_models(Box::new(Listeners(Arc::clone(&offered))))
    .with_browsers(scripted);
    let endpoint = channels::ProviderName::parse("dead").unwrap();
    worker
        .handle(channels::Command::AttachEndpoint {
            name: endpoint.clone(),
            base_url: refusing_url(),
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            admit: vec![MODEL.to_owned()],
            tuning: channels::EndpointTuning::default(),
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
    city::write_rules(
        dir.path(),
        &Address::parse(LAB).unwrap(),
        "confidential = false\nwrite = \"everything\"\nbrowser = true\n",
    )
    .unwrap();
    let dispatched = worker.handle(channels::Command::Dispatch {
        addr: Address::parse(LAB).unwrap(),
        task: "Answer.".to_owned(),
        goal: "one turn with the browser this worker was handed".to_owned(),
        mode: kernel::Mode::PlanGoal,
        idem: idem(b"dispatch"),
        session: Some(kernel::SessionName::parse("s1").unwrap()),
        effort: None,
        model: None,
    });

    let names = offered.lock().unwrap().clone();
    assert!(
        names.iter().any(|name| name == HANDED),
        "the model was never offered {HANDED}: offered {names:?}, and the dispatch answered {dispatched:?}"
    );
}
