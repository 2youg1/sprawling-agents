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
/// number of chat completions, registered the way a person would
/// register one so nothing reaches the worker by a door the production
/// path does not have. Tests that only need it to answer bind it as
/// `_provider`; tests about what went out on the wire read `bodies()`,
/// the one place a claim about the wire can be checked.
pub(super) struct FakeProvider {
    seen: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    _handle: std::thread::JoinHandle<()>,
}

/// What this provider does with the first chat request it reads.
/// [`FirstChat::Dropped`] stages the transient disconnect a test
/// otherwise cannot hit on purpose: the request arrives whole, the
/// connection closes, no byte of an answer goes back. It spends no
/// scripted reply and joins no record, so every later turn answers
/// the question it was written for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FirstChat {
    Answered,
    Dropped,
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
/// Anthropic-format third party does - the shape a city has to attach
/// on the ids the person declared.
#[cfg(test)]
pub(super) fn fake_openai(models: &[&str], replies: Vec<String>) -> (String, FakeProvider) {
    fake_openai_with(models, replies, FirstChat::Answered)
}

/// The same provider, told what to do with the first chat request.
#[cfg(test)]
pub(super) fn fake_openai_with(
    models: &[&str],
    replies: Vec<String>,
    first_chat: FirstChat,
) -> (String, FakeProvider) {
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
    // The last reply repeats: a test says what the interesting turns
    // are, not how many turns the loop will take. The script is shared
    // because every connection is served on a thread of its own.
    let script = std::sync::Arc::new(std::sync::Mutex::new((replies.into_iter(), String::new())));
    let drops = std::sync::atomic::AtomicBool::new(first_chat == FirstChat::Dropped);
    let dropping = std::sync::Arc::new(drops);
    let handle = std::thread::spawn(move || {
        // **One thread per accepted connection.** Two handdown runs go
        // into two lanes and call this provider at once; serving them
        // one after the other leaves the second client waiting on a
        // socket nobody reads, a timing window the city never has
        // against a real provider. One bad socket ends that socket, not
        // the server; the bound is there so a listener that is
        // genuinely gone stops rather than spins.
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
            let recorder = std::sync::Arc::clone(&recorder);
            let script = std::sync::Arc::clone(&script);
            let dropping = std::sync::Arc::clone(&dropping);
            let list = list.clone();
            std::thread::spawn(move || {
                use std::io::ErrorKind;
                let mut head = String::new();
                let mut buf = [0u8; 4096];
                let mut whole = false;
                loop {
                    let n = match std::io::Read::read(&mut stream, &mut buf) {
                        // An end of stream, and nothing else, ends the
                        // reading; a signal or a would-block leaves the
                        // rest of a request still on its way.
                        Ok(0) => break,
                        Ok(n) => n,
                        Err(e)
                            if matches!(
                                e.kind(),
                                ErrorKind::Interrupted | ErrorKind::WouldBlock
                            ) =>
                        {
                            continue;
                        }
                        Err(_) => break,
                    };
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
                // for a header terminator and the reply for nothing at
                // all - so a socket carrying no request stayed off the
                // record and still spent a scripted reply, and every
                // turn after it answered the question before it.
                if !whole {
                    return;
                }
                let a_chat = !head.starts_with("GET ");
                // The request landed; the answer never starts.
                if a_chat && dropping.swap(false, std::sync::atomic::Ordering::SeqCst) {
                    return;
                }
                // The whole exchange, headers included: a test about
                // what went out on the wire needs the headers too.
                recorder.lock().unwrap().push(head.clone());
                let (status, body) = if a_chat {
                    let mut script = script.lock().unwrap();
                    let (chats, last) = &mut *script;
                    let next = chats.next().inspect(|reply| last.clone_from(reply));
                    (200, next.unwrap_or_else(|| last.clone()))
                } else if serves_a_list {
                    (200, list)
                } else {
                    (404, "{\"error\":\"no such route\"}".to_owned())
                };
                let response = format!(
                    "HTTP/1.1 {status} OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = std::io::Write::write_all(&mut stream, response.as_bytes());
            });
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

/// One reply that calls a named tool with the arguments given. The
/// arguments are the tool's real contract, because an invented shape
/// here once hid the fact that no canary edit had ever landed on disk.
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
        tuning: channels::EndpointTuning::default(),
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"attach"),
    })?;
    worker.handle(channels::Command::SelectModel {
        endpoint: channels::ProviderName::parse("house").unwrap(),
        model: model.to_owned(),
        tag: kernel::ModelTag::Main,
        context_tokens: kernel::Window::new(32_768),
        max_output_tokens: kernel::Ceiling::new(4_096),
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

/// A control surface that listens for interrupts and nothing else.
///
/// The three sinks are one value and one injection, so a test that
/// cares about steers says what it does not listen for rather than
/// reaching for a setter of its own.
pub(super) fn only_interrupts(
    source: std::sync::Arc<dyn Fn(RunId) -> Interrupt + Send + Sync>,
) -> Serving {
    Serving {
        deltas: std::sync::Arc::new(|_delta| {}),
        machine: std::sync::Arc::new(|_found| {}),
        interrupts: source,
    }
}

/// **The product defect this knob was built to catch.** One chat
/// request lands and the connection closes before a byte comes back,
/// and both handdown runs still arrive: a failure that completed no
/// exchange is retriable, so the default `UntilHalted` asks again.
/// While no provider failure was ever retriable, this one drop froze
/// the run and the parent joined nothing.
#[test]
fn a_dropped_call_is_asked_again_and_both_handdowns_still_come_back() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let graph = serde_json::json!({ "op": "lay_out", "nodes": [
        { "room": "lab/writer", "goal": "write it up", "done_check": "the page exists",
          "stop": "when the page exists", "depends_on": ["lab/reader"] },
        { "room": "lab/reader", "goal": "read the meter", "stop": "when it is written down",
          "done_check": "a number is written down" },
    ] });
    let (base_url, _provider) = fake_openai_with(
        &["m-local"],
        vec![
            completion_with("splitting it up", "workshop", "tu_1", graph.clone()),
            completion("waiting on a person", None),
            completion_with("splitting it up", "workshop", "tu_2", graph),
            completion("done", None),
        ],
        FirstChat::Dropped,
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(channels::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: channels::TemplateName::parse("minimal").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();
    let room = Address::parse("lab/room1").unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: room.clone(),
            task: "get it measured and written up".to_owned(),
            goal: "a page with a number in it, then stop".to_owned(),
            mode: kernel::Mode::PlanGoal,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();
    let joined = worker.joins.get(&room).map_or(0, |j| j.artifacts().count());
    assert_eq!(
        joined, 2,
        "a dropped connection cost a handdown: the call was never asked again"
    );
}
