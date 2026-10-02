// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The desktop connector is a tool this binary carries and starts itself
//! (sprawling-SPEC.md section 8-4d).
//!
//! A city starts the real `sprawling` executable as `sprawling desktop`
//! for a building whose rules ask for the desktop, with no `[[mcp]]` row
//! anywhere; the test reads the tool names the model was offered. That
//! the six arrive is the proof that the verb answered the city's own
//! `initialize` and `tools/list` over the child's pipes.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use kernel::{
    Address, AxError, ContentBlock, IdemKey, Model, ModelRequest, ModelReturn, RunId, Seq,
};
use sprawling::assembly;

const LAB: &str = "lab";
const MODEL: &str = "scripted";

/// The six tools, as the model is offered them under the city's own
/// label for the server.
const OFFERED: [&str; 6] = [
    "desktop_desktop_windows",
    "desktop_desktop_snapshot",
    "desktop_desktop_act",
    "desktop_desktop_screenshot",
    "desktop_desktop_record",
    "desktop_desktop_clipboard",
];

#[test]
fn a_building_given_the_desktop_is_offered_its_six_tools_from_this_binary() {
    let dir = tempfile::tempdir().unwrap();
    assembly::init_city(dir.path()).unwrap();
    let offered = Arc::new(Mutex::new(Vec::new()));
    let mut worker = accounting::worker::RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        assembly::hands(gateway::Custodian::in_memory()),
    )
    .unwrap()
    .with_models(Box::new(Listeners(Arc::clone(&offered))))
    .with_desktop_program(this_binary);
    let endpoint = wire::ProviderName::parse("dead").unwrap();
    worker
        .handle(wire::Command::AttachEndpoint {
            name: endpoint.clone(),
            base_url: refusing_url(),
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            admit: vec![MODEL.to_owned()],
            tuning: wire::EndpointTuning::default(),
            idem: idem(b"attach"),
        })
        .unwrap();
    worker
        .handle(wire::Command::SelectModel {
            endpoint,
            model: MODEL.to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: kernel::Window::new(32_768),
            max_output_tokens: kernel::Ceiling::new(1_024),
            input: None,
            idem: idem(b"select"),
        })
        .unwrap();
    worker
        .handle(wire::Command::CreateBuilding {
            addr: Address::parse(LAB).unwrap(),
            template: wire::TemplateName::parse("minimal").unwrap(),
            idem: idem(b"create"),
        })
        .unwrap();
    city::write_rules(
        dir.path(),
        &Address::parse(LAB).unwrap(),
        "confidential = false\nwrite = \"everything\"\ndesktop = true\n",
    )
    .unwrap();
    let dispatched = worker.handle(wire::Command::Dispatch {
        addr: Address::parse(LAB).unwrap(),
        task: "Answer.".to_owned(),
        goal: "one turn with this machine's desktop".to_owned(),
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        idem: idem(b"dispatch"),
        session: Some(kernel::SessionName::parse("s1").unwrap()),
        effort: None,
        model: None,
    });

    let names = offered.lock().unwrap().clone();
    let missing: Vec<&str> = OFFERED
        .into_iter()
        .filter(|tool| !names.iter().any(|name| name == tool))
        .collect();
    assert!(
        missing.is_empty(),
        "the model was never offered {missing:?}: offered {names:?}, and the dispatch answered {dispatched:?}"
    );
}

/// The executable cargo built for this package, standing in for
/// `std::env::current_exe`, which in a test names the test harness.
fn this_binary() -> std::io::Result<PathBuf> {
    // boundary-ok: the city starts this executable as its own child in production, so the test must hand it a real one; every check still enters through RunWorker
    Ok(PathBuf::from(env!("CARGO_BIN_EXE_sprawling")))
}

/// Keeps the name of every tool it is offered, in its tool list or its
/// dormant index, and ends the run on its first turn.
struct Listening(Arc<Mutex<Vec<String>>>);

impl Model for Listening {
    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
        self.0.lock().unwrap().extend(
            req.chat
                .tools
                .iter()
                .map(|tool| tool.name.as_str().to_owned()),
        );
        // A tool outside the mode's core is offered as a line of the
        // dormant index (`crates/runtime/Spec.lean` §8-60).
        self.0.lock().unwrap().extend(
            req.chat
                .system
                .iter()
                .filter_map(|block| block.text.split_once("Dormant,"))
                .flat_map(|(_, index)| index.lines().skip(1))
                .filter_map(|line| line.strip_prefix("- "))
                .map(|entry| {
                    entry
                        .split_once(':')
                        .map_or(entry, |(name, _)| name)
                        .to_owned()
                }),
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
