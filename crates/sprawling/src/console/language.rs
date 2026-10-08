// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one typed line asks for, read against the slash table
//! (`crates/sprawling/spec/Console.lean` §8-11, `wire::slash`).
//!
//! The slash verbs are `wire::Slash`, the one table the WebUI reads
//! too; this module owns no spelling of its own. `/wire` carries every
//! other verb the socket knows, projected from `wire::COMMAND_NAMES` and
//! `wire::QUERY_NAMES` and spelled by [`snake`], so a command renamed on
//! the wire is renamed here in the same build.
//!
//! The judgement is pure and total: no clock, no socket, no ledger; the
//! line's idempotency key arrives as a parameter. A line nobody can
//! classify comes back as [`Line::Unknown`] carrying the nearest verbs.
//!
//! The point a reader most often gets wrong: a line that does not begin
//! with `/` is work for the chosen room rather than an unknown verb, and
//! the argument of a `/wire` verb is the wire's own JSON.

use kernel::{Address, IdemKey};
use wire::Slash;

/// The two Commands whose `Command::idem()` is `None`, which a socket
/// cannot carry: `PutSecret` has no wire form at all, and `Auth` is
/// proved in the handshake rather than sent as a command.
const OFF_THE_SOCKET: [&str; 2] = ["PutSecret", "Auth"];

/// The Commands a `/wire` line can become, in wire order.
pub(super) fn carried_commands() -> impl Iterator<Item = &'static str> {
    wire::COMMAND_NAMES
        .iter()
        .copied()
        .filter(|name| !OFF_THE_SOCKET.contains(name))
}

/// Whether a verb with `--all` covers the whole city or the room's
/// building.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reach {
    Building,
    City,
}

/// What one line asked for.
#[derive(Debug, PartialEq)]
pub(crate) enum Line {
    /// An empty line, which is not a question.
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
    /// The remote door: open it, pair a device, close it, list or
    /// revoke devices. Never a frame, so nothing on the wire can ask.
    Remote(crate::outside::console::RemoteLine),
    /// A `/wire` verb, already built into the frame it names.
    Frame(Box<wire::ClientFrame>),
    /// Anything that does not begin with `/`: work for the chosen room,
    /// in the words the person used.
    Work(String),
    /// A verb this console does not have, with the nearest ones it does.
    Unknown {
        verb: String,
        nearest: Vec<String>,
    },
    /// A verb this console has, with a body the wire cannot read; the
    /// reason is the reader's own, which names the field at fault.
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

/// Every verb `/wire` answers to.
///
/// **A projection, never a second list**: derived from
/// [`carried_commands`] and `wire::QUERY_NAMES`.
pub(crate) fn wire_verbs() -> Vec<String> {
    carried_commands()
        .map(snake)
        .chain(wire::QUERY_NAMES.iter().map(|name| snake(name)))
        .collect()
}

/// The slash verbs the terminal offers, as typed.
pub(crate) fn slash_verbs() -> impl Iterator<Item = Slash> {
    Slash::ALL
        .into_iter()
        .filter(|verb| verb.offered().in_the_cli())
}

/// The names closest to something a person typed: those sharing the
/// longest prefix with it that any name shares at all.
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

/// The argument after a verb, when there is one.
fn argument(tail: &str) -> Option<String> {
    (!tail.is_empty()).then(|| tail.to_owned())
}

/// Reads one line; a `/wire` Command whose body carries no `idem` gets
/// `idem`, the key this console minted for the line.
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
        Slash::Remote => Line::Remote(crate::outside::console::parse(tail)),
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

/// A wire verb and the rest of the line, turned into the frame it names.
///
/// The body is JSON because the wire is JSON: a second argument grammar
/// here would be a second description of every type on the wire.
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

/// `{"<key>": value}`, the shape every level of a wire frame takes.
fn keyed(key: &str, value: serde_json::Value) -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::from_iter([(key.to_owned(), value)]))
}

fn malformed(verb: &str, err: &serde_json::Error) -> Line {
    Line::Malformed {
        verb: verb.to_owned(),
        reason: err.to_string(),
    }
}

/// What `/help` prints: every slash verb the terminal offers, one a
/// line, then the `/wire` verbs.
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

/// The ask frame for one question. The console answers in place, so
/// every question it asks carries the same number: nothing waits to be
/// paired.
fn asked(query: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "ask": { "ask_id": 0, "query": query } })
}
