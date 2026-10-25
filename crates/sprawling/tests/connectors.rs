// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A run offered the MCP tools the worker was handed
//! (`crates/accounting/Spec.lean` section 2), and found through `tool`
//! however many a server lists (`crates/runtime/Spec.lean` §8-60).
//!
//! The building names a server whose command does not exist on any
//! machine, so its tool can reach the model only through the connectors
//! the worker received. A worker that still started its own servers
//! would fail to start this one and offer the model nothing.

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
// The city is formed with an in-memory vault, because these tests are
// about the worker and must not write to the credential service of the
// machine that runs them.
use accounting::worker::genesis::{Adopt, form};
use sprawling::assembly;

const LAB: &str = "lab";
const MODEL: &str = "scripted";
/// What the scripted server lists: one tool, which the city offers the
/// model under the server's label.
const LISTING: &str = "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"tools\":[{\"name\":\"ping\",\"description\":\"answer with pong\",\"inputSchema\":{\"type\":\"object\"}}]}}";
const OFFERED: &str = "apps_ping";
/// The namespace every tool of the server named `apps` is filed under.
const NAMESPACE: &str = "apps";

/// A listing of two hundred tools, one per pairing of a verb and a
/// noun, the size of a large real server's.
fn two_hundred_tools() -> String {
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

/// The names the dormant index of `req` lists: a namespace's name and
/// count, or a skill's `skill <name>`.
fn dormant_names(req: &ModelRequest) -> Vec<String> {
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
        // A tool outside the mode's core is offered under its namespace,
        // one line of the dormant index (`crates/runtime/Spec.lean` §8-60).
        self.0.lock().unwrap().extend(dormant_names(req));
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

/// Answers every server with the listing it holds, without starting
/// anything.
struct Scripted(String);

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

    /// Keeps no connection, so there is nothing a new key makes stale.
    fn invalidate(&self, _reference: &kernel::SecretRef) {}
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

/// Appends a `[[mcp]]` table naming a server no machine has to the
/// building layer.
fn name_an_absent_server(city_root: &std::path::Path) {
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

/// Forms a city with one building that names the server `apps`, and
/// dispatches one run of `models` there with `connectors`; answers how
/// the dispatch ended, for a failure message.
fn dispatch_one_run(
    models: Box<dyn accounting::ModelFactory + Send>,
    connectors: Box<dyn accounting::Connectors + Send + Sync>,
) -> String {
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
            idem: idem(b"create"),
        })
        .unwrap();
    name_an_absent_server(dir.path());
    let dispatched = worker.handle(wire::Command::Dispatch {
        addr: Address::parse(LAB).unwrap(),
        task: "Answer.".to_owned(),
        goal: "one turn with the tools this worker was handed".to_owned(),
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        idem: idem(b"dispatch"),
        session: Some(kernel::SessionName::parse("s1").unwrap()),
        effort: None,
        model: None,
    });
    format!("{dispatched:?}")
}

#[test]
fn a_run_is_offered_the_tools_the_worker_was_handed() {
    let offered = Arc::new(Mutex::new(Vec::new()));
    let dispatched = dispatch_one_run(
        Box::new(Listeners(Arc::clone(&offered))),
        Box::new(Scripted(LISTING.to_owned())),
    );

    assert!(
        offered
            .lock()
            .unwrap()
            .iter()
            .any(|name| *name == format!("{NAMESPACE} (1)")),
        "the model was never offered {OFFERED} under {NAMESPACE}: offered {:?}; the dispatch \
         answered {dispatched}",
        offered.lock().unwrap()
    );
}

/// A server of two hundred tools takes one line of the dormant index,
/// and `tool` `find` names the one a few words describe among its first
/// eight hits (`crates/runtime/Spec.lean` §8-60, D110).
#[test]
fn two_hundred_tools_of_one_server_take_one_index_line_and_are_found() {
    let seen = Arc::new(Mutex::new(Seen::default()));
    let dispatched = dispatch_one_run(
        Box::new(Callers(Calling {
            seen: Arc::clone(&seen),
            tool: "tool",
            args: serde_json::json!({"op": "find", "query": "create issue"}),
        })),
        Box::new(Scripted(two_hundred_tools())),
    );

    let seen = seen.lock().unwrap();
    let lines: Vec<&String> = seen
        .index
        .iter()
        .filter(|name| name.starts_with(NAMESPACE))
        .collect();
    assert_eq!(
        lines,
        [&format!("{NAMESPACE} (200)")],
        "index {:?}; the dispatch answered {dispatched}",
        seen.index
    );
    let found = seen.read.first().map(String::as_str).unwrap_or_default();
    let hits: Vec<&str> = found
        .lines()
        .filter(|line| line.starts_with("- "))
        .collect();
    assert!(
        hits.len() <= 8
            && hits
                .iter()
                .any(|hit| hit.starts_with("- apps_create_issue:")),
        "find answered {found:?}; the dispatch answered {dispatched}"
    );
}

const CALL: &str = "{\"jsonrpc\":\"2.0\",\"id\":0,\"method\":\"tools/call\",\"params\":{\"name\":\"ping\",\"arguments\":{}}}";

/// A server whose one tool answers with a long converted document.
struct Answering(String);

impl accounting::Connectors for Answering {
    fn connect(
        &self,
        server: &kernel::McpServer,
        _write_root: &std::path::Path,
        confidential: bool,
        _resolve: &gateway::SecretResolver,
    ) -> Result<(Vec<agent_protocols::McpTool>, accounting::Reached), AxError> {
        let answer = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 0,
            "result": { "content": [{ "type": "text", "text": self.0 }], "isError": false },
        })
        .to_string();
        let tools =
            agent_protocols::tools_from(&server.label, &agent_protocols::Rpc::read(LISTING)?)?
                .into_iter()
                .map(|entry| {
                    let mut outbound = agent_protocols::ScriptedOutbound::new();
                    outbound.answer(CALL, &answer)?;
                    agent_protocols::McpTool::new(
                        entry.meta,
                        entry.remote,
                        Box::new(outbound),
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

    /// Keeps no connection, so there is nothing a new key makes stale.
    fn invalidate(&self, _reference: &kernel::SecretRef) {}
}

/// What a calling model saw: the dormant index of its first request,
/// and every tool result it read back.
#[derive(Default)]
struct Seen {
    index: Vec<String>,
    read: Vec<String>,
}

/// Makes one tool call, then records what it read back.
#[derive(Clone)]
struct Calling {
    seen: Arc<Mutex<Seen>>,
    tool: &'static str,
    args: serde_json::Value,
}

impl Model for Calling {
    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
        let results: Vec<String> = req
            .chat
            .messages
            .iter()
            .flat_map(|message| &message.content)
            .filter_map(|block| match block {
                ContentBlock::ToolResult { content, .. } => Some(content.clone()),
                ContentBlock::Text { .. }
                | ContentBlock::Thinking { .. }
                | ContentBlock::RedactedThinking { .. }
                | ContentBlock::ToolUse { .. }
                | ContentBlock::Image(_) => None,
            })
            .collect();
        if !results.is_empty() {
            self.seen.lock().unwrap().read.extend(results);
            return Ok(ModelReturn::bare(
                kernel::model::message_payload(&[ContentBlock::Text {
                    text: "done".to_owned(),
                }])?,
                Vec::new(),
            ));
        }
        self.seen.lock().unwrap().index = dormant_names(req);
        let name = kernel::ToolName::parse(self.tool)?;
        let input = kernel::Payload::of(&self.args)?;
        Ok(ModelReturn::bare(
            kernel::model::message_payload(&[ContentBlock::ToolUse {
                id: "call-1".to_owned(),
                name: name.clone(),
                input: input.clone(),
            }])?,
            vec![kernel::ToolCall {
                id: "call-1".to_owned(),
                name,
                args: input,
            }],
        ))
    }
}

struct Callers(Calling);

impl accounting::ModelFactory for Callers {
    fn build(
        &self,
        _chosen: &gateway::Chosen<'_>,
        _redemption: gateway::Redemption,
    ) -> Result<Box<dyn Model + Send>, AxError> {
        Ok(Box::new(self.0.clone()))
    }
}

/// A converted document longer than the window reaches the model as
/// the pipeline's window over it, not whole (`crates/runtime/Spec.lean` §8-27-10).
#[test]
fn a_long_connector_answer_reaches_the_model_packaged() {
    let document = "The quarter closed with every account reconciled. ".repeat(1_000);
    let seen = Arc::new(Mutex::new(Seen::default()));
    let dispatched = dispatch_one_run(
        Box::new(Callers(Calling {
            seen: Arc::clone(&seen),
            tool: OFFERED,
            args: serde_json::json!({}),
        })),
        Box::new(Answering(document.clone())),
    );

    let read = &seen.lock().unwrap().read;
    assert_eq!(
        read.len(),
        1,
        "one result read back; the dispatch answered {dispatched}"
    );
    assert!(
        read[0].len() < document.len(),
        "the model read {} bytes of a {}-byte document",
        read[0].len(),
        document.len()
    );
}
