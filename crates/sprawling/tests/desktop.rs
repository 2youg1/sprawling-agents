// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The desktop connector is a tool this binary carries and starts itself
//! (sprawling-SPEC.md section 8-4d).
//!
//! Both tests start the real `sprawling` executable. The first speaks
//! to `sprawling desktop` over its pipes the way the city's own client
//! does; the second lets a city start it for a building whose rules ask
//! for the desktop, with no `[[mcp]]` row anywhere, and reads the tool
//! names the model was offered.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
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
fn the_desktop_verb_answers_an_initialize_and_a_tools_list_over_its_own_pipes() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sprawling"))
        .arg("desktop")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the binary starts");
    {
        let requests = child.stdin.as_mut().expect("the child was given pipes");
        for line in [
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":\
             {\"protocolVersion\":\"2025-06-18\",\"capabilities\":{},\"clientInfo\":\
             {\"name\":\"sprawling\",\"version\":\"0.0.7\"}}}",
            "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\",\"params\":{}}",
            "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\",\"params\":{}}",
        ] {
            writeln!(requests, "{line}").expect("the request reaches the child");
        }
        requests.flush().expect("the request is not left buffered");
    }
    // Closing stdin ends the child's read loop, so the process finishes
    // on its own and this test cannot hang on a pipe nobody closed.
    let stdout = child.stdout.take().expect("the child was given pipes");
    let mut answers = BufReader::new(stdout).lines();
    drop(child.stdin.take());

    let opened = answers.next().map(|line| line.unwrap());
    let opened: serde_json::Value =
        serde_json::from_str(opened.as_deref().unwrap_or("null")).unwrap();
    assert_eq!(opened["id"], 1, "initialize is answered: {opened}");
    assert_eq!(opened["result"]["serverInfo"]["name"], "sprawling-desktop");

    let listed: serde_json::Value =
        serde_json::from_str(&answers.next().expect("tools/list is answered").unwrap()).unwrap();
    let names: Vec<&str> = listed["result"]["tools"]
        .as_array()
        .expect("the list is an array")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    assert_eq!(
        names,
        vec![
            "desktop.windows",
            "desktop.snapshot",
            "desktop.act",
            "desktop.screenshot",
            "desktop.record",
            "desktop.clipboard",
        ]
    );
    assert!(
        answers.next().is_none(),
        "a notification was answered, which would put the pipe one line out of step"
    );
    let ended = child.wait().expect("the child ends when its input closes");
    assert!(ended.success(), "the server ended with {ended}");
}

#[test]
fn a_building_given_the_desktop_is_offered_its_six_tools_from_this_binary() {
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
    .with_desktop_program(this_binary);
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
        "confidential = false\nwrite = \"everything\"\ndesktop = true\n",
    )
    .unwrap();
    let dispatched = worker.handle(channels::Command::Dispatch {
        addr: Address::parse(LAB).unwrap(),
        task: "Answer.".to_owned(),
        goal: "one turn with this machine's desktop".to_owned(),
        mode: kernel::Mode::PlanGoal,
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
    Ok(PathBuf::from(env!("CARGO_BIN_EXE_sprawling")))
}

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
