// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::{BufRead as _, Read as _, Write as _};

use super::*;
use crate::worker::RunWorker;
use crate::worker::fixture::*;

mod rounds;

/// What the loopback supplier answers one request with.
#[derive(Clone, Copy)]
enum Reply {
    /// The method's ordinary answer.
    Answer,
    /// This status and no body.
    Status(u16),
    /// A 200 whose body is not text, so the answer is lost after the
    /// server took the request.
    Unread,
}

/// One request the loopback supplier was sent: the key its
/// `x-api-key` header carried, and the JSON-RPC message.
#[derive(Debug, Clone)]
struct Sent {
    key: Option<String>,
    request: serde_json::Value,
}

impl Sent {
    fn method(&self) -> &str {
        self.request["method"].as_str().unwrap_or("")
    }
}

type Decide = fn(&Sent) -> Reply;

/// A supplier on a loopback port offering one tool, `remote`, with
/// `schema` as its `inputSchema`. `decide` picks each answer; every
/// request is kept in arrival order.
fn hosted(
    remote: &'static str,
    schema: serde_json::Value,
    decide: Decide,
) -> (String, Arc<Mutex<Vec<Sent>>>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/mcp", listener.local_addr().unwrap());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::clone(&seen);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut reader = std::io::BufReader::new(stream.unwrap());
            let (mut key, mut length) = (None, 0);
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                let lower = line.to_ascii_lowercase();
                if let Some(value) = lower.strip_prefix("content-length:") {
                    length = value.trim().parse::<usize>().unwrap();
                }
                if lower.starts_with("x-api-key:") {
                    key = line
                        .split_once(':')
                        .map(|(_, value)| value.trim().to_owned());
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            let sent = Sent {
                key,
                request: serde_json::from_slice(&body).unwrap(),
            };
            let reply = decide(&sent);
            kept.lock().unwrap().push(sent.clone());
            let result = match sent.method() {
                "initialize" => serde_json::json!({
                    "protocolVersion": agent_protocols::PROTOCOL_VERSION,
                    "serverInfo": {"name": "supplier"}
                }),
                "tools/list" => serde_json::json!({
                    "tools": [{"name": remote, "inputSchema": schema}]
                }),
                _ => serde_json::json!({"content": [{"type": "text", "text": "one result"}]}),
            };
            let (status, answer) = match reply {
                Reply::Status(status) => (status, Vec::new()),
                Reply::Unread => (200, vec![0xff]),
                Reply::Answer if sent.request.get("id").is_none() => (202, Vec::new()),
                Reply::Answer => (
                    200,
                    serde_json::to_vec(&serde_json::json!({
                        "jsonrpc": "2.0", "id": sent.request["id"], "result": result
                    }))
                    .unwrap(),
                ),
            };
            let head = format!(
                "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                answer.len()
            );
            let stream = reader.get_mut();
            stream.write_all(head.as_bytes()).unwrap();
            stream.write_all(&answer).unwrap();
        }
    });
    (url, seen)
}

/// The schema of a supplier that takes `q`, and an optional `count`.
fn vendor_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {"q": {"type": "string"}, "count": {"type": "number"}},
        "required": ["q"]
    })
}

fn account(id: &str) -> ProviderAccount {
    ProviderAccount {
        id: ServerLabel::parse(id).unwrap(),
        reference: Some(kernel::SecretRef::parse(&format!("secret:search/{id}")).unwrap()),
        header: Some("x-api-key".to_owned()),
    }
}

/// A custom supplier at `url` whose remote tool is `vendor_search`,
/// sending from `accounts` in their order.
fn vendor(url: &str, accounts: &[&str]) -> SearchConfiguration {
    let id = ServerLabel::parse("vendor").unwrap();
    SearchConfiguration::Custom {
        selected: id.clone(),
        suppliers: vec![SearchSupplier {
            id,
            url: url.to_owned(),
            remote: "vendor_search".to_owned(),
            query_field: "q".to_owned(),
            objective_field: None,
            count_field: Some("count".to_owned()),
            accounts: accounts.iter().map(|id| account(id)).collect(),
        }],
    }
}

/// A worker whose vault holds `key-<id>` under `secret:search/<id>`
/// for each of `ids`.
fn worker_holding(city_root: &Path, ids: &[&str]) -> RunWorker {
    init_city(city_root).unwrap();
    let mut worker =
        RunWorker::new(city_root, runtime::diagnostics::Diagnostics::off(), hands()).unwrap();
    for id in ids {
        worker
            .handle(wire::Command::PutSecret {
                realm: "search".to_owned(),
                name: (*id).to_owned(),
                value: kernel::Sealed::new(Box::new(format!("key-{id}"))),
            })
            .unwrap();
    }
    worker
}

fn offered(worker: &RunWorker, root: &Path, configuration: &SearchConfiguration) -> WebSearch {
    worker
        .laying("")
        .unwrap()
        .web_search(
            &kernel::BuildingPolicy::new(false),
            configuration,
            root,
            RunId::CITY,
        )
        .unwrap()
        .unwrap()
}

fn search(tool: &WebSearch, args: serde_json::Value) -> Result<ToolOutcome, AxError> {
    tool.invoke(&ToolCall {
        id: "search".to_owned(),
        name: tool.meta().name.clone(),
        args: kernel::Payload::of(&args).unwrap(),
    })
}

fn calls_from(seen: &Mutex<Vec<Sent>>, key: &str) -> Vec<Sent> {
    seen.lock()
        .unwrap()
        .iter()
        .filter(|sent| sent.key.as_deref() == Some(key))
        .cloned()
        .collect()
}

/// The call reaches the selected supplier's url as one `tools/call` of
/// its remote tool, with each argument under the remote name the
/// mapping gives it and the key of the first account in its header.
#[test]
fn web_search_calls_the_selected_supplier_with_the_mapped_arguments_and_its_key() {
    let dir = tempfile::tempdir().unwrap();
    let worker = worker_holding(dir.path(), &["alpha"]);
    let (url, seen) = hosted("vendor_search", vendor_schema(), |_| Reply::Answer);
    let tool = offered(&worker, dir.path(), &vendor(&url, &["alpha"]));

    let answer = search(
        &tool,
        serde_json::json!({"query": "rust ownership", "num_results": 3}),
    )
    .unwrap();

    assert_eq!(
        answer.result.as_map().get("content"),
        Some(&serde_json::json!([{"type": "text", "text": "one result"}])),
        "the supplier's content comes back as it gave it"
    );
    let sent = seen.lock().unwrap().clone();
    let called: Vec<&Sent> = sent.iter().filter(|s| s.method() == "tools/call").collect();
    assert_eq!(called.len(), 1);
    assert_eq!(
        called[0].request["params"],
        serde_json::json!({"name": "vendor_search", "arguments": {"q": "rust ownership", "count": 3}})
    );
    assert!(
        sent.iter().all(|s| s.key.as_deref() == Some("key-alpha")),
        "every request carries the account's key: {sent:?}"
    );
}

/// Absent `[search]` offers the declared default, Exa, reached
/// anonymously; a confidential building and `Off` are offered nothing.
#[test]
fn web_search_defaults_to_exa_and_is_not_offered_when_confidential_or_off() {
    let dir = tempfile::tempdir().unwrap();
    let worker = worker_holding(dir.path(), &[]);
    let tool = offered(&worker, dir.path(), &SearchConfiguration::Default);
    assert_eq!(tool.supplier, city::default_search_supplier().unwrap());
    assert_eq!(
        declaration(&tool.supplier, &tool.supplier.accounts[0]).unwrap(),
        kernel::McpServer {
            label: ServerLabel::parse("exa").unwrap(),
            transport: kernel::McpTransport::Http {
                url: "https://mcp.exa.ai/mcp".to_owned(),
                headers: Vec::new(),
            },
        }
    );
    assert_eq!(
        tool.meta().params.as_map().get("required"),
        Some(&serde_json::json!(["query", "objective"]))
    );

    let laying = worker.laying("").unwrap();
    for (confidential, configuration) in [
        (true, SearchConfiguration::Default),
        (false, SearchConfiguration::Off),
    ] {
        assert!(
            laying
                .web_search(
                    &kernel::BuildingPolicy::new(confidential),
                    &configuration,
                    dir.path(),
                    RunId::CITY,
                )
                .unwrap()
                .is_none()
        );
    }
}

/// A custom supplier that cannot be reached is the failed result: no
/// request goes to the default supplier or anywhere else.
#[test]
fn web_search_never_falls_back_when_the_custom_supplier_cannot_be_reached() {
    struct Recording(
        crate::worker::mcp::Residents,
        Arc<Mutex<Vec<kernel::McpServer>>>,
    );
    impl crate::Connectors for Recording {
        fn connect(
            &self,
            server: &kernel::McpServer,
            write_root: &Path,
            confidential: bool,
            resolve: &gateway::SecretResolver,
        ) -> Result<(Vec<agent_protocols::McpTool>, crate::Reached), AxError> {
            self.1.lock().unwrap().push(server.clone());
            self.0.connect(server, write_root, confidential, resolve)
        }
        fn invalidate(&self, reference: &kernel::SecretRef) {
            self.0.invalidate(reference);
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let asked = Arc::new(Mutex::new(Vec::new()));
    let worker = worker_holding(dir.path(), &["alpha"]).with_connectors(Box::new(Recording(
        crate::worker::mcp::Residents::default(),
        Arc::clone(&asked),
    )));
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/mcp", listener.local_addr().unwrap());
    drop(listener);
    let tool = offered(&worker, dir.path(), &vendor(&url, &["alpha"]));

    let failed = search(&tool, serde_json::json!({"query": "anything"})).unwrap_err();

    assert_eq!(failed.code(), &AxCode::ToolUnavailable);
    let asked = asked.lock().unwrap();
    assert!(!asked.is_empty());
    assert!(
        asked.iter().all(|server| matches!(
            &server.transport,
            kernel::McpTransport::Http { url: reached, .. } if *reached == url
        )),
        "only the selected supplier was asked: {asked:?}"
    );
}

/// A mapping the remote schema does not take is refused before any
/// `tools/call` is sent.
#[test]
fn web_search_refuses_a_mapping_the_remote_tool_does_not_take_before_the_call() {
    let dir = tempfile::tempdir().unwrap();
    let worker = worker_holding(dir.path(), &["alpha"]);
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "q": {"type": "string"},
            "count": {"type": "number"},
            "purpose": {"type": "string"}
        },
        "required": ["q", "purpose"]
    });
    let (url, seen) = hosted("vendor_search", schema, |_| Reply::Answer);
    let tool = offered(&worker, dir.path(), &vendor(&url, &["alpha"]));

    let refused = search(&tool, serde_json::json!({"query": "anything"})).unwrap_err();

    assert_eq!(refused.code(), &AxCode::ToolUnavailable);
    assert!(
        refused.subject().contains("purpose"),
        "{}",
        refused.subject()
    );
    assert!(
        seen.lock()
            .unwrap()
            .iter()
            .all(|s| s.method() != "tools/call"),
        "nothing was sent to a tool the mapping does not fit"
    );
}

/// The tool joins the bench a run is laid out with: an ordinary
/// building's model is told about `web_search` under the default
/// supplier, without anything being sent to it.
#[test]
fn an_ordinary_building_offers_web_search_to_its_model() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: kernel::Address::parse("lab/room1").unwrap(),
            task: "look nothing up".to_owned(),
            goal: "answer".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();
    assert!(
        provider
            .bodies()
            .iter()
            .any(|body| body.contains("web_search")),
        "the tool table the model is given carries web_search: {:?}",
        provider.bodies()
    );
}
