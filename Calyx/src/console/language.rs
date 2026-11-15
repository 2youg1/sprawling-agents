// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, IdemKey};
use wire::Slash;

const OFF_THE_SOCKET: [&str; 2] = ["PutSecret", "Auth"];

pub(super) fn carried_commands() -> impl Iterator<Item = &'static str> {
    wire::COMMAND_NAMES
        .iter()
        .copied()
        .filter(|name| !OFF_THE_SOCKET.contains(name))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reach {
    Building,
    City,
}

#[derive(Debug, PartialEq)]
pub(crate) enum Line {
    Nothing,
    Help,
    OpenWeb,
    Serving,
    Quit,
    Select(Address),
    New,
    Stop,
    Halt(Reach),
    Release(Reach),
    Model(Option<String>),
    Effort(Option<String>),
    Approve(Option<String>),
    Deny(Option<String>),
    Acp(Option<String>),
    Remote(String),
    Frame(Box<wire::ClientFrame>),
    Work(String),
    Unknown {
        verb: String,
        nearest: Vec<String>,
    },
    Malformed {
        verb: String,
        reason: String,
    },
}

pub(crate) fn snake(camel: &str) -> String {
    let mut out = String::with_capacity(camel.len().saturating_add(4));
    for (index, ch) in camel.char_indices() {
        if ch.is_ascii_uppercase() && index > 0 {
            out.push('_');
        }
        out.extend(ch.to_lowercase());
    }
    out
}

pub(crate) fn wire_verbs() -> Vec<String> {
    carried_commands()
        .map(snake)
        .chain(wire::QUERY_NAMES.iter().map(|name| snake(name)))
        .collect()
}

pub(crate) fn slash_verbs() -> impl Iterator<Item = Slash> {
    Slash::ALL
        .into_iter()
        .filter(|verb| verb.offered().in_the_cli())
}

fn nearest(typed: &str, known: &[String]) -> Vec<String> {
    let chars: Vec<char> = typed.chars().collect();
    for length in (1..=chars.len().min(4)).rev() {
        let head: String = chars.iter().take(length).collect();
        let mut close: Vec<String> = known
            .iter()
            .filter(|name| name.starts_with(&head))
            .cloned()
            .collect();
        if !close.is_empty() {
            close.truncate(6);
            return close;
        }
    }
    Vec::new()
}

fn argument(tail: &str) -> Option<String> {
    (!tail.is_empty()).then(|| tail.to_owned())
}

pub(crate) fn parse(line: &str, idem: IdemKey) -> Line {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Line::Nothing;
    }
    let Some(rest) = trimmed.strip_prefix('/') else {
        return Line::Work(trimmed.to_owned());
    };
    let (verb, tail) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
    let tail = tail.trim();
    let Some(slash) = Slash::parse(verb).filter(|slash| slash.offered().in_the_cli()) else {
        let known: Vec<String> = slash_verbs()
            .map(|verb| verb.spelling().trim_start_matches('/').to_owned())
            .collect();
        return Line::Unknown {
            verb: verb.to_owned(),
            nearest: nearest(verb, &known),
        };
    };
    let reach = || {
        if tail == "--all" {
            Reach::City
        } else {
            Reach::Building
        }
    };
    match slash {
        Slash::Help => Line::Help,
        Slash::Web => Line::OpenWeb,
        Slash::Serving => Line::Serving,
        Slash::Quit => Line::Quit,
        Slash::New => Line::New,
        Slash::Stop => Line::Stop,
        Slash::Halt => Line::Halt(reach()),
        Slash::Release => Line::Release(reach()),
        Slash::Model => Line::Model(argument(tail)),
        Slash::Effort => Line::Effort(argument(tail)),
        Slash::Approve => Line::Approve(argument(tail)),
        Slash::Deny => Line::Deny(argument(tail)),
        Slash::Acp => Line::Acp(argument(tail)),
        Slash::Remote => Line::Remote(tail.to_owned()),
        Slash::Room => match Address::parse(tail) {
            Ok(addr) => Line::Select(addr),
            Err(_) => Line::Unknown {
                verb: format!("room {tail}"),
                nearest: vec!["room <building>/<room>".to_owned()],
            },
        },
        Slash::Wire => {
            let (verb, body) = tail.split_once(char::is_whitespace).unwrap_or((tail, ""));
            wire_frame(verb, body.trim(), idem)
        }
    }
}

fn wire_frame(verb: &str, tail: &str, idem: IdemKey) -> Line {
    let body = if tail.is_empty() {
        Ok(serde_json::Value::Null)
    } else {
        serde_json::from_str::<serde_json::Value>(tail)
    };
    let known_command = carried_commands().find(|name| snake(name) == verb);
    let known_query = wire::QUERY_NAMES.iter().find(|name| snake(name) == verb);
    let framed = match (known_command, known_query, body) {
        (None, None, _) => {
            return Line::Unknown {
                verb: format!("wire {verb}"),
                nearest: nearest(verb, &wire_verbs()),
            };
        }
        (_, _, Err(err)) => return malformed(verb, &err),
        (Some(name), _, Ok(mut body)) => {
            if let serde_json::Value::Object(fields) = &mut body {
                fields
                    .entry("idem")
                    .or_insert_with(|| serde_json::Value::String(idem.to_string()));
            }
            keyed("command", keyed(&snake(name), body))
        }
        (None, Some(name), Ok(serde_json::Value::Null)) => {
            asked(serde_json::Value::String(snake(name)))
        }
        (None, Some(name), Ok(body)) => asked(keyed(&snake(name), body)),
    };
    match serde_json::from_value::<wire::ClientFrame>(framed) {
        Ok(frame) => Line::Frame(Box::new(frame)),
        Err(err) => malformed(verb, &err),
    }
}

fn keyed(key: &str, value: serde_json::Value) -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::from_iter([(key.to_owned(), value)]))
}

fn malformed(verb: &str, err: &serde_json::Error) -> Line {
    Line::Malformed {
        verb: verb.to_owned(),
        reason: err.to_string(),
    }
}

pub(crate) fn help(room: &Address) -> String {
    let rows: String = slash_verbs()
        .map(|verb| {
            let typed = format!("{} {}", verb.spelling(), verb.takes());
            format!("  {typed:<26}{}\n", verb.says())
        })
        .collect();
    format!(
        "\n  plain lines go to {}\n\n{rows}\n  /wire verbs              {}\n",
        room.as_str(),
        wire_verbs().join(", ")
    )
}

fn asked(query: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "ask": { "ask_id": 0, "query": query } })
}
