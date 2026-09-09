// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

/// Lays a building's rules where the city reads them.
///
/// Through `city::building_path` rather than by joining a file name:
/// a fixture that spells the path itself is a second authority for
/// where the rules live, and it goes on passing after the real one
/// has moved.
pub(super) fn lay_rules(city_root: &Path, building: &str, text: &str) {
    let addr = Address::parse(building).unwrap();
    let file = city::building_path(city_root, &addr);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, text).unwrap();
}

/// A loopback provider that answers a model list and then a fixed
/// number of chat completions. These tests register it the way a
/// person would, so nothing here reaches the worker by a door the
/// production path does not have.
/// A fake provider and what it was asked. Tests that only need it to
/// answer bind it as `_provider`; tests about what went out on the
/// wire read `bodies()`, because the request body is the only place
/// a claim about the wire can be checked.
pub(super) struct FakeProvider {
    seen: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    _handle: std::thread::JoinHandle<()>,
}

impl FakeProvider {
    pub(super) fn bodies(&self) -> Vec<String> {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .map(|head| {
                head.split_once("\r\n\r\n")
                    .map_or(String::new(), |(_, body)| body.to_owned())
            })
            .collect()
    }

    pub(super) fn exchanges(&self) -> Vec<String> {
        self.seen.lock().unwrap().clone()
    }
}

/// An empty `models` list means this provider serves no model list at
/// all: `GET .../models` answers 404, the way a gateway or an
/// Anthropic-format third party does. That is the shape a city has to
/// attach on the ids the person declared.
#[cfg(test)]
pub(super) fn fake_openai(models: &[&str], replies: Vec<String>) -> (String, FakeProvider) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let serves_a_list = !models.is_empty();
    let list = serde_json::json!({
        "data": models
            .iter()
            .map(|id| serde_json::json!({ "id": id }))
            .collect::<Vec<_>>(),
    })
    .to_string();
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorder = std::sync::Arc::clone(&seen);
    let handle = std::thread::spawn(move || {
        // The last reply repeats: a test says what the interesting
        // turns are, not how many turns the loop will take.
        let mut chats = replies.into_iter().peekable();
        let mut last = String::new();
        // One bad socket ends that socket, not the server. A client
        // is free to reset a connection at any point, including
        // before the accept completes, and a server that returns on
        // it takes every later turn of the script with it. The bound
        // is there so a listener that is genuinely gone stops rather
        // than spins.
        let mut refused = 0_u32;
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else {
                refused = refused.saturating_add(1);
                if refused > 64 {
                    return;
                }
                continue;
            };
            refused = 0;
            let mut head = String::new();
            let mut buf = [0u8; 4096];
            let mut whole = false;
            // A read that errors ends this request, not the loop
            // that serves the next one.
            while let Ok(n) = std::io::Read::read(&mut stream, &mut buf) {
                if n == 0 {
                    break;
                }
                head.push_str(&String::from_utf8_lossy(&buf[..n]));
                if let Some(end) = head.find("\r\n\r\n") {
                    let want = head
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length: ")
                                .and_then(|v| v.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    let body_seen = head.len().saturating_sub(end.saturating_add(4));
                    if body_seen >= want {
                        whole = true;
                        break;
                    }
                }
            }
            // What counts as a request is settled here and nowhere
            // else. It used to be settled twice - the record asked
            // for a header terminator and the reply asked for
            // nothing at all - so a socket that carried no request
            // stayed off the record and still spent a scripted
            // reply, and every turn after it answered the question
            // before it.
            if !whole {
                continue;
            }
            // The whole exchange, headers included: a test about
            // what went out on the wire needs the headers too.
            recorder.lock().unwrap().push(head.clone());
            let (status, body) = if head.starts_with("GET ") {
                if serves_a_list {
                    (200, list.clone())
                } else {
                    (404, "{\"error\":\"no such route\"}".to_owned())
                }
            } else {
                (200, {
                    match chats.next() {
                        Some(reply) => {
                            last.clone_from(&reply);
                            reply
                        }
                        None => last.clone(),
                    }
                })
            };
            let response = format!(
                "HTTP/1.1 {status} OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = std::io::Write::write_all(&mut stream, response.as_bytes());
        }
    });
    (
        format!("http://{addr}/v1"),
        FakeProvider {
            seen,
            _handle: handle,
        },
    )
}

/// One OpenAI chat completion: `calls` decides whether the turn asks
/// for the edit tool or ends. The call uses the edit tool's real
/// contract (create form), so these tests exercise the same argument
/// shape a model is told about - an invented shape here once hid the
/// fact that no canary edit had ever landed on disk.
/// One reply that calls a named tool with the arguments given.
pub(super) fn completion_with(
    text: &str,
    tool: &str,
    id: &str,
    arguments: serde_json::Value,
) -> String {
    serde_json::json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": text,
                "tool_calls": [{
                    "id": id,
                    "type": "function",
                    "function": { "name": tool, "arguments": arguments.to_string() },
                }],
            },
            "finish_reason": "tool_calls",
        }],
        "usage": { "prompt_tokens": 12, "completion_tokens": 5 },
    })
    .to_string()
}

pub(super) fn completion(text: &str, call: Option<(&str, &str)>) -> String {
    let mut message = serde_json::json!({ "role": "assistant", "content": text });
    let mut finish = "stop";
    if let Some((id, path)) = call {
        let arguments = serde_json::json!({
            "path": path,
            "base_version": "new",
            "old": "",
            "new": "noted\n",
        })
        .to_string();
        message["tool_calls"] = serde_json::json!([{
            "id": id,
            "type": "function",
            "function": { "name": "edit", "arguments": arguments },
        }]);
        finish = "tool_calls";
    }
    serde_json::json!({
        "choices": [{ "message": message, "finish_reason": finish }],
        "usage": { "prompt_tokens": 12, "completion_tokens": 5 },
    })
    .to_string()
}

/// A worker with one endpoint attached and one model chosen, exactly
/// as the settings page would leave it.
pub(super) fn worker_with_provider(
    city_root: &Path,
    base_url: &str,
    model: &str,
) -> Result<RunWorker, AxError> {
    let mut worker = RunWorker::new(
        city_root,
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )?;
    worker.handle(channels::Command::AttachEndpoint {
        name: channels::ProviderName::parse("house").unwrap(),
        base_url: base_url.to_owned(),
        dialect: kernel::DialectKind::OpenAi,
        secret: None,
        auth_header: None,
        admit: Vec::new(),
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"attach"),
    })?;
    worker.handle(channels::Command::SelectModel {
        endpoint: channels::ProviderName::parse("house").unwrap(),
        model: model.to_owned(),
        tag: kernel::ModelTag::Main,
        context_tokens: 32_768,
        max_output_tokens: 4_096,
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"select"),
    })?;
    Ok(worker)
}

/// One completion that calls a named tool with the given arguments.
/// Separate from `completion` because that helper hard-codes the edit
/// tool's shape, and a tool面 test is about a different tool.
pub(super) fn tool_completion(text: &str, id: &str, tool: &str, args: serde_json::Value) -> String {
    serde_json::json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": text,
                "tool_calls": [{
                    "id": id,
                    "type": "function",
                    "function": { "name": tool, "arguments": args.to_string() },
                }],
            },
            "finish_reason": "tool_calls",
        }],
        "usage": { "prompt_tokens": 12, "completion_tokens": 5 },
    })
    .to_string()
}

pub(super) const PLAN_TWO_FREE_ROWS: &str = concat!(
    "| # | Item | Weight | Needs | Status | Evidence |\n",
    "|---|---|---|---|---|---|\n",
    "| 1 | wire the kiln | 1 |  | Not started |  |\n",
    "| 2 | glaze tests | 1 |  | Not started |  |\n",
);

/// Three rows nothing blocks, which is a ready set of three: what a
/// pursuit takes the whole of rather than one node at a time
/// (sprawling-SPEC 8-46-4).
pub(super) const PLAN_THREE_FREE_ROWS: &str = concat!(
    "| # | Item | Weight | Needs | Status | Evidence |\n",
    "|---|---|---|---|---|---|\n",
    "| 1 | wire the kiln | 1 |  | Not started |  |\n",
    "| 2 | glaze tests | 1 |  | Not started |  |\n",
    "| 3 | stack the shelves | 1 |  | Not started |  |\n",
);

pub(super) const PLAN_ONE_FREE_ROW: &str = concat!(
    "| # | Item | Weight | Needs | Status | Evidence |\n",
    "|---|---|---|---|---|---|\n",
    "| 1 | wire the kiln | 1 |  | Not started |  |\n",
);

/// Answers the one thing waiting, as the person would. Delegation
/// now asks before it hands anything down, so a test that wants a
/// delegate has to say yes first - which is the point of the door.
pub(super) fn allow_the_one_pending_item(worker: &mut RunWorker) -> kernel::ClusterKey {
    let item = worker
        .governance
        .pending
        .values()
        .next()
        .cloned()
        .expect("exactly one thing is waiting");
    worker
        .handle(channels::Command::Approve {
            item: item.id.clone(),
            verdict: kernel::PolicyVerdict::Allow,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"allow"),
        })
        .unwrap();
    item.cluster_key
}
