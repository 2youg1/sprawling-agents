// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::io::Write as _;
use std::sync::{Arc, Mutex};

use kernel::{
    Address, AxError, ContentBlock, IdemKey, Model, ModelRequest, ModelReturn, RunId, Seq,
};

use accounting::worker::genesis::{Adopt, form};
use sprawling::assembly;

pub(super) const LAB: &str = "lab";
pub(super) const MODEL: &str = "scripted";

pub(super) const LISTING: &str = "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"tools\":[{\"name\":\"ping\",\"description\":\"answer with pong\",\"inputSchema\":{\"type\":\"object\"}}]}}";
pub(super) const OFFERED: &str = "apps_ping";

pub(super) const NAMESPACE: &str = "apps";

pub(super) fn two_hundred_tools() -> String {
    const VERBS: [&str; 10] = [
        "create", "get", "list", "update", "delete", "search", "close", "merge", "add", "remove",
    ];
    const NOUNS: [&str; 20] = [
        "issue",
        "pull_request",
        "comment",
        "label",
        "branch",
        "release",
        "repository",
        "milestone",
        "review",
        "workflow",
        "gist",
        "team",
        "project",
        "tag",
        "commit",
        "file",
        "secret",
        "webhook",
        "discussion",
        "collaborator",
    ];
    let tools: Vec<serde_json::Value> = VERBS
        .iter()
        .flat_map(|verb| {
            NOUNS.iter().map(move |noun| {
                serde_json::json!({
                    "name": format!("{verb}_{noun}"),
                    "description": format!("{verb} a {} on the forge", noun.replace('_', " ")),
                    "inputSchema": {"type": "object"},
                })
            })
        })
        .collect();
    serde_json::json!({"jsonrpc": "2.0", "id": 1, "result": {"tools": tools}}).to_string()
}

pub(super) fn dormant_names(req: &ModelRequest) -> Vec<String> {
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
        })
        .collect()
}

pub(super) struct Listening(Arc<Mutex<Vec<String>>>);

impl Model for Listening {
    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
        self.0.lock().unwrap().extend(
            req.chat
                .tools
                .iter()
                .map(|tool| tool.name.as_str().to_owned()),
        );

        self.0.lock().unwrap().extend(dormant_names(req));
        Ok(ModelReturn::bare(
            kernel::model::message_payload(&[ContentBlock::Text {
                text: "done".to_owned(),
            }])?,
            Vec::new(),
        ))
    }
}

pub(super) struct Listeners(Arc<Mutex<Vec<String>>>);

impl accounting::ModelFactory for Listeners {
    fn build(
        &self,
        _chosen: &gateway::Chosen<'_>,
        _redemption: gateway::Redemption,
    ) -> Result<Box<dyn Model + Send>, AxError> {
        Ok(Box::new(Listening(Arc::clone(&self.0))))
    }
}

pub(super) struct Scripted(String);

pub(super) type Listing = Arc<Mutex<Option<String>>>;
pub(super) struct Revisions(Listing);

impl accounting::Connectors for Revisions {
    fn connect(
        &self,
        server: &kernel::McpServer,
        _write_root: &std::path::Path,
        confidential: bool,
        _resolve: &gateway::SecretResolver,
    ) -> Result<(Vec<agent_protocols::McpTool>, accounting::Reached), AxError> {
        let listing = self.0.lock().unwrap().clone().ok_or_else(|| {
            AxError::failure(
                kernel::AxCode::ToolUnavailable,
                "list a scripted connector",
                server.label.as_str(),
            )
            .with_recovery("restore the scripted listing")
        })?;
        let tools =
            agent_protocols::tools_from(&server.label, &agent_protocols::Rpc::read(&listing)?)?
                .into_iter()
                .map(|entry| {
                    agent_protocols::McpTool::new(
                        entry.meta,
                        entry.remote,
                        Box::new(agent_protocols::ScriptedOutbound::new()),
                        confidential,
                    )
                })
                .collect::<Result<Vec<_>, AxError>>()?;
        Ok((
            tools,
            accounting::Reached::Connected(agent_protocols::Handshake {
                protocol_version: agent_protocols::PROTOCOL_VERSION.to_owned(),
                server: "revisions".to_owned(),
            }),
        ))
    }

    fn invalidate(&self, _reference: &kernel::SecretRef) {}
}

impl accounting::Connectors for Scripted {
    fn connect(
        &self,
        server: &kernel::McpServer,
        _write_root: &std::path::Path,
        confidential: bool,
        _resolve: &gateway::SecretResolver,
    ) -> Result<(Vec<agent_protocols::McpTool>, accounting::Reached), AxError> {
        let tools =
            agent_protocols::tools_from(&server.label, &agent_protocols::Rpc::read(&self.0)?)?
                .into_iter()
                .map(|entry| {
                    agent_protocols::McpTool::new(
                        entry.meta,
                        entry.remote,
                        Box::new(agent_protocols::ScriptedOutbound::new()),
                        confidential,
                    )
                })
                .collect::<Result<Vec<_>, AxError>>()?;
        let opened = agent_protocols::Handshake {
            protocol_version: agent_protocols::PROTOCOL_VERSION.to_owned(),
            server: "scripted".to_owned(),
        };
        Ok((tools, accounting::Reached::Connected(opened)))
    }

    fn invalidate(&self, _reference: &kernel::SecretRef) {}
}

pub(super) fn refusing_url() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    format!("http://127.0.0.1:{port}/v1")
}

pub(super) fn idem(what: &[u8]) -> IdemKey {
    IdemKey::derive(&RunId::CITY, Seq::FIRST, what)
}

pub(super) fn name_an_absent_server(city_root: &std::path::Path) {
    let path = city::config_path(
        city_root,
        &Address::parse(LAB).unwrap(),
        city::Layer::Building,
    )
    .unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(
            b"\n[[mcp]]\nlabel = \"apps\"\ncommand = \"sprawling-no-such-server\"\nargs = []\n",
        )
        .unwrap();
}

pub(super) fn listing(tools: &[(&str, &str)]) -> String {
    let tools: Vec<serde_json::Value> = tools
        .iter()
        .map(|(name, description)| {
            serde_json::json!({
                "name": name,
                "description": description,
                "inputSchema": {"type": "object"},
            })
        })
        .collect();
    serde_json::json!({"jsonrpc": "2.0", "id": 1, "result": {"tools": tools}}).to_string()
}

pub(super) fn revision_city(
    initial: &str,
) -> (
    tempfile::TempDir,
    accounting::worker::RunWorker,
    Listing,
    Arc<Mutex<Vec<String>>>,
) {
    let listing = Arc::new(Mutex::new(Some(initial.to_owned())));
    let offered = Arc::new(Mutex::new(Vec::new()));
    let (dir, worker) = lab_city(
        Box::new(Listeners(Arc::clone(&offered))),
        Box::new(Revisions(Arc::clone(&listing))),
    );
    (dir, worker, listing, offered)
}

pub(super) fn dispatch_one_run(
    models: Box<dyn accounting::ModelFactory + Send>,
    connectors: Box<dyn accounting::Connectors + Send + Sync>,
) -> String {
    let (_dir, mut worker) = lab_city(models, connectors);
    dispatch(&mut worker, "s1")
}

pub(super) fn dispatch(worker: &mut accounting::worker::RunWorker, session: &str) -> String {
    let dispatched = worker.handle(wire::Command::Dispatch {
        addr: Address::parse(LAB).unwrap(),
        task: "Answer.".to_owned(),
        goal: "one turn with the tools this worker was handed".to_owned(),
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        idem: idem(session.as_bytes()),
        session: Some(kernel::SessionName::parse(session).unwrap()),
        effort: None,
        model: None,
    });
    format!("{dispatched:?}")
}

pub(super) fn lab_city(
    models: Box<dyn accounting::ModelFactory + Send>,
    connectors: Box<dyn accounting::Connectors + Send + Sync>,
) -> (tempfile::TempDir, accounting::worker::RunWorker) {
    let dir = tempfile::tempdir().unwrap();
    form(
        dir.path(),
        Adopt::Nothing,
        assembly::hands(gateway::Custodian::in_memory()),
    )
    .unwrap();
    let mut worker = accounting::worker::RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        assembly::hands(gateway::Custodian::in_memory()),
    )
    .unwrap()
    .with_models(models)
    .with_connectors(connectors);
    let endpoint = wire::ProviderName::parse("dead").unwrap();
    worker
        .handle(wire::Command::AttachEndpoint {
            name: endpoint.clone(),
            base_url: refusing_url(),
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            admit: vec![MODEL.to_owned()],
            named: Vec::new(),
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
            rules: None,
            source: None,
            idem: idem(b"create"),
        })
        .unwrap();
    name_an_absent_server(dir.path());
    (dir, worker)
}

mod drift;
mod offering;
