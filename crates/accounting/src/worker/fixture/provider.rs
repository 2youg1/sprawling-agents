// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The loopback provider a fixture talks to, and the two shapes of
//! script it answers with.

/// A loopback provider that answers a model list and then a fixed
/// number of chat completions, registered the way a person would
/// register one so nothing reaches the worker by a door the production
/// path does not have. Tests that only need it to answer bind it as
/// `_provider`; tests about what went out on the wire read `bodies()`,
/// the one place a claim about the wire can be checked.
pub(in crate::worker) struct FakeProvider {
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
pub(in crate::worker) enum FirstChat {
    Answered,
    Dropped,
}

impl FakeProvider {
    pub(in crate::worker) fn bodies(&self) -> Vec<String> {
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

    pub(in crate::worker) fn exchanges(&self) -> Vec<String> {
        self.seen.lock().unwrap().clone()
    }
}

/// An empty `models` list means this provider serves no model list at
/// all: `GET .../models` answers 404, the way a gateway or an
/// Anthropic-format third party does - the shape a city has to attach
/// on the ids the person declared.
#[cfg(test)]
pub(in crate::worker) fn fake_openai(
    models: &[&str],
    replies: Vec<String>,
) -> (String, FakeProvider) {
    fake_openai_with(models, replies, FirstChat::Answered)
}

/// What a chat request is answered with.
///
/// **Two shapes, because two kinds of test ask different questions.** A
/// script in arrival order is what one conversation needs: its requests
/// are one run's turns one after another, so the order *is* the
/// scenario. A script keyed by what the request names is what several
/// runs at once need, and the reason is a flake this fixture had: two
/// node runs call the provider concurrently with the room above them,
/// the machine decides which request arrives first, and an order-based
/// script then hands one node the reply written for the other - the
/// node reports `E_EVIDENCE_MISSING` against a done check it was never
/// given the answer for, and a test that passed a minute ago fails.
///
/// A keyed route holds a phrase the request carries when it is *that*
/// work: a request naming exactly one route takes that route's replies,
/// and one naming two or none - the room above them, whose prefix holds
/// both - takes the ordered fallback. Which is the whole rule, and it
/// makes a node's answer independent of who asked first.
enum Script {
    InOrder {
        queued: std::vec::IntoIter<String>,
        last: String,
    },
    Routed {
        routes: Vec<Route>,
        fallback: Route,
    },
}

/// One route of a [`Script::Routed`]: the phrase, and what to answer.
struct Route {
    key: String,
    queued: std::vec::IntoIter<String>,
    last: String,
}

impl Route {
    fn new(key: &str, replies: Vec<String>) -> Route {
        Route {
            key: key.to_owned(),
            queued: replies.into_iter(),
            last: String::new(),
        }
    }

    /// The next reply, or the last one again.
    ///
    /// The last reply repeats because a test says what the interesting
    /// turns are, not how many turns the loop will take.
    fn take(&mut self) -> String {
        let next = self
            .queued
            .next()
            .inspect(|reply| self.last.clone_from(reply));
        next.unwrap_or_else(|| self.last.clone())
    }
}

impl Script {
    fn answer(&mut self, request: &str) -> String {
        match self {
            Script::InOrder { queued, last } => {
                let next = queued.next().inspect(|reply| last.clone_from(reply));
                next.unwrap_or_else(|| last.clone())
            }
            Script::Routed { routes, fallback } => {
                let named: Vec<usize> = routes
                    .iter()
                    .enumerate()
                    .filter(|(_, route)| request.contains(&route.key))
                    .map(|(index, _)| index)
                    .collect();
                match named.as_slice() {
                    [only] => routes[*only].take(),
                    _ => fallback.take(),
                }
            }
        }
    }
}

/// A provider that answers each piece of work by what the request names.
///
/// `routes` pairs a phrase with the replies for the run whose request
/// carries it; `fallback` is the ordered script for a request that names
/// two routes or none (`Script::Routed` says why that is the room above
/// them). Use it for a fixture with more than one run in flight.
#[cfg(test)]
pub(in crate::worker) fn fake_openai_routed(
    models: &[&str],
    routes: Vec<(&str, Vec<String>)>,
    fallback: Vec<String>,
) -> (String, FakeProvider) {
    fake_openai_routed_with(models, routes, fallback, FirstChat::Answered)
}

/// The same, told what to do with the first chat request.
#[cfg(test)]
pub(in crate::worker) fn fake_openai_routed_with(
    models: &[&str],
    routes: Vec<(&str, Vec<String>)>,
    fallback: Vec<String>,
    first_chat: FirstChat,
) -> (String, FakeProvider) {
    serve(models, routed(routes, fallback), first_chat, unpaced())
}

/// What a paced provider calls with each whole request, headers
/// included, before it answers: an instrument holds a run at its model
/// call by blocking here, and makes one kind of request slow by
/// sleeping.
pub(in crate::worker) type Pace = std::sync::Arc<dyn Fn(&str) + Send + Sync>;

/// A routed provider that hands every request to `pace` first.
#[cfg(test)]
pub(in crate::worker) fn fake_openai_paced(
    models: &[&str],
    routes: Vec<(&str, Vec<String>)>,
    fallback: Vec<String>,
    pace: Pace,
) -> (String, FakeProvider) {
    serve(models, routed(routes, fallback), FirstChat::Answered, pace)
}

fn routed(routes: Vec<(&str, Vec<String>)>, fallback: Vec<String>) -> Script {
    Script::Routed {
        routes: routes
            .into_iter()
            .map(|(key, replies)| Route::new(key, replies))
            .collect(),
        fallback: Route::new("", fallback),
    }
}

fn unpaced() -> Pace {
    std::sync::Arc::new(|_: &str| {})
}

/// The same provider, told what to do with the first chat request.
#[cfg(test)]
pub(in crate::worker) fn fake_openai_with(
    models: &[&str],
    replies: Vec<String>,
    first_chat: FirstChat,
) -> (String, FakeProvider) {
    let script = Script::InOrder {
        queued: replies.into_iter(),
        last: String::new(),
    };
    serve(models, script, first_chat, unpaced())
}

/// One listener serving `script`, on a port of its own.
#[cfg(test)]
fn serve(
    models: &[&str],
    script: Script,
    first_chat: FirstChat,
    pace: Pace,
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
    // The script is shared because every connection is served on a
    // thread of its own.
    let script = std::sync::Arc::new(std::sync::Mutex::new(script));
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
            let pace = std::sync::Arc::clone(&pace);
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
                // else. Settled twice - the record asking for a header
                // terminator and the reply for nothing at all - a socket
                // carrying no request would stay off the record and still
                // spend a scripted reply, and every turn after it would
                // answer the question before it.
                if !whole {
                    return;
                }
                let a_chat = !head.starts_with("GET ");
                // The request landed; the answer never starts.
                if a_chat && dropping.swap(false, std::sync::atomic::Ordering::SeqCst) {
                    return;
                }
                pace(&head);
                // The whole exchange, headers included: a test about
                // what went out on the wire needs the headers too.
                recorder.lock().unwrap().push(head.clone());
                let (status, body) = if a_chat {
                    // Chosen under the lock, so two requests never take
                    // one reply between them.
                    (200, script.lock().unwrap().answer(&head))
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
pub(in crate::worker) fn completion_with(
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

pub(in crate::worker) fn completion(text: &str, call: Option<(&str, &str)>) -> String {
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
