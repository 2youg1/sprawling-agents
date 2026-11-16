// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

pub(in crate::worker) struct FakeProvider {
    seen: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    listing: std::sync::Arc<std::sync::atomic::AtomicBool>,
    _handle: std::thread::JoinHandle<()>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::worker) enum FirstChat {
    Answered,
    Dropped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::worker) enum Reply {
    Status {
        status: u16,
        headers: Vec<(String, String)>,
        body: String,
    },

    Lost,
}

impl Reply {
    pub(in crate::worker) fn answered(body: String) -> Reply {
        Reply::Status {
            status: 200,
            headers: Vec::new(),
            body,
        }
    }

    pub(in crate::worker) fn refused(status: u16, error: &serde_json::Value) -> Reply {
        Reply::Status {
            status,
            headers: Vec::new(),
            body: serde_json::json!({ "error": error }).to_string(),
        }
    }

    pub(in crate::worker) fn with_header(self, name: &str, value: &str) -> Reply {
        match self {
            Reply::Status {
                status,
                mut headers,
                body,
            } => {
                headers.push((name.to_owned(), value.to_owned()));
                Reply::Status {
                    status,
                    headers,
                    body,
                }
            }
            Reply::Lost => Reply::Lost,
        }
    }
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

    pub(in crate::worker) fn stop_listing(&self) {
        self.listing
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

#[cfg(test)]
pub(in crate::worker) fn fake_openai(
    models: &[&str],
    replies: Vec<String>,
) -> (String, FakeProvider) {
    fake_openai_with(models, replies, FirstChat::Answered)
}

enum Script {
    InOrder {
        queued: std::vec::IntoIter<Reply>,
        last: Reply,
    },
    Routed {
        routes: Vec<Route>,
        fallback: Route,
    },
}

struct Route {
    key: String,
    queued: std::vec::IntoIter<Reply>,
    last: Reply,
}

impl Route {
    fn new(key: &str, replies: Vec<Reply>) -> Route {
        Route {
            key: key.to_owned(),
            queued: replies.into_iter(),
            last: Reply::answered(String::new()),
        }
    }

    fn take(&mut self) -> Reply {
        let next = self
            .queued
            .next()
            .inspect(|reply| self.last.clone_from(reply));
        next.unwrap_or_else(|| self.last.clone())
    }
}

impl Script {
    fn answer(&mut self, request: &str) -> Reply {
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

#[cfg(test)]
pub(in crate::worker) fn fake_openai_routed(
    models: &[&str],
    routes: Vec<(&str, Vec<String>)>,
    fallback: Vec<String>,
) -> (String, FakeProvider) {
    fake_openai_routed_with(models, routes, fallback, FirstChat::Answered)
}

#[cfg(test)]
pub(in crate::worker) fn fake_openai_routed_with(
    models: &[&str],
    routes: Vec<(&str, Vec<String>)>,
    fallback: Vec<String>,
    first_chat: FirstChat,
) -> (String, FakeProvider) {
    serve(models, routed(routes, fallback), first_chat, unpaced())
}

pub(in crate::worker) type Pace = std::sync::Arc<dyn Fn(&str) + Send + Sync>;

#[cfg(test)]
pub(in crate::worker) fn fake_openai_paced(
    models: &[&str],
    routes: Vec<(&str, Vec<String>)>,
    fallback: Vec<String>,
    pace: Pace,
) -> (String, FakeProvider) {
    serve(models, routed(routes, fallback), FirstChat::Answered, pace)
}

#[cfg(test)]
pub(in crate::worker) fn fake_openai_keyed(
    models: &[&str],
    keys: Vec<(&str, Vec<Reply>)>,
    pace: Pace,
) -> (String, FakeProvider) {
    let script = Script::Routed {
        routes: keys
            .into_iter()
            .map(|(key, replies)| Route::new(key, replies))
            .collect(),
        fallback: Route::new(
            "",
            vec![Reply::refused(
                401,
                &serde_json::json!({ "code": "invalid_api_key" }),
            )],
        ),
    };
    serve(models, script, FirstChat::Answered, pace)
}

fn routed(routes: Vec<(&str, Vec<String>)>, fallback: Vec<String>) -> Script {
    let replies = |bodies: Vec<String>| bodies.into_iter().map(Reply::answered).collect();
    Script::Routed {
        routes: routes
            .into_iter()
            .map(|(key, bodies)| Route::new(key, replies(bodies)))
            .collect(),
        fallback: Route::new("", replies(fallback)),
    }
}

fn unpaced() -> Pace {
    std::sync::Arc::new(|_: &str| {})
}

#[cfg(test)]
pub(in crate::worker) fn fake_openai_with(
    models: &[&str],
    replies: Vec<String>,
    first_chat: FirstChat,
) -> (String, FakeProvider) {
    let script = Script::InOrder {
        queued: replies
            .into_iter()
            .map(Reply::answered)
            .collect::<Vec<_>>()
            .into_iter(),
        last: Reply::answered(String::new()),
    };
    serve(models, script, first_chat, unpaced())
}

#[cfg(test)]
fn listed_row(id: &str) -> serde_json::Value {
    if id.ends_with("-thinks") {
        serde_json::json!({
            "id": id,
            "effort": { "supported_levels": ["low", "medium", "high", "xhigh", "max"] },
        })
    } else if id.ends_with("-stated") {
        serde_json::json!({
            "id": id,
            "context_length": LISTED_WINDOW,
            "top_provider": { "max_completion_tokens": LISTED_CEILING },
        })
    } else {
        serde_json::json!({ "id": id })
    }
}

#[cfg(test)]
pub(in crate::worker) const LISTED_WINDOW: u64 = 131_072;

#[cfg(test)]
pub(in crate::worker) const LISTED_CEILING: u64 = 16_384;

#[cfg(test)]
fn serve(
    models: &[&str],
    script: Script,
    first_chat: FirstChat,
    pace: Pace,
) -> (String, FakeProvider) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let listing = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(!models.is_empty()));
    let lists = std::sync::Arc::clone(&listing);
    let list = serde_json::json!({
        "data": models
            .iter()
            .copied()
            .map(listed_row)
            .collect::<Vec<_>>(),
    })
    .to_string();
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorder = std::sync::Arc::clone(&seen);

    let script = std::sync::Arc::new(std::sync::Mutex::new(script));
    let drops = std::sync::atomic::AtomicBool::new(first_chat == FirstChat::Dropped);
    let dropping = std::sync::Arc::new(drops);
    let handle = std::thread::spawn(move || {
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
            let lists = std::sync::Arc::clone(&lists);
            std::thread::spawn(move || {
                use std::io::ErrorKind;
                let mut head = String::new();
                let mut buf = [0u8; 4096];
                let mut whole = false;
                loop {
                    let n = match std::io::Read::read(&mut stream, &mut buf) {
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

                if !whole {
                    return;
                }
                let a_chat = !head.starts_with("GET ");

                if a_chat && dropping.swap(false, std::sync::atomic::Ordering::SeqCst) {
                    return;
                }
                pace(&head);

                recorder.lock().unwrap().push(head.clone());
                let reply = if a_chat {
                    script.lock().unwrap().answer(&head)
                } else if lists.load(std::sync::atomic::Ordering::SeqCst) {
                    Reply::answered(list)
                } else {
                    Reply::refused(404, &serde_json::json!("no such route"))
                };
                let Reply::Status {
                    status,
                    headers,
                    body,
                } = reply
                else {
                    return;
                };
                let extra: String = headers
                    .iter()
                    .map(|(name, value)| format!("{name}: {value}\r\n"))
                    .collect();
                let response = format!(
                    "HTTP/1.1 {status} OK\r\ncontent-type: application/json\r\n{extra}content-length: {}\r\nconnection: close\r\n\r\n{body}",
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
            listing,
            _handle: handle,
        },
    )
}

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
            "file_path": path,
            "old_string": "",
            "new_string": "noted\n",
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
