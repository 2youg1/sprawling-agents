// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The console's vocabulary, and the judgement of one typed line into a
//! [`Line`] (`crates/sprawling/spec/Console.lean` §8-11).
//!
//! This module owns the console's own verbs — `CONTROL`, the six that
//! never reach the wire — and owns nothing else about any verb. Every
//! other verb is a projection of the Commands a socket can carry and of
//! `wire::QUERY_NAMES`, spelled by [`snake`], so a command renamed on
//! the wire is renamed here in the same build and a hand-written table
//! never becomes a second vocabulary.
//!
//! The judgement is pure and total: no clock, no socket, no ledger, and
//! no error type; the line's idempotency key arrives as a parameter. A line nobody can classify comes back as
//! [`Line::Unknown`] carrying the nearest verbs, because a console that
//! refused to classify a line would have nothing to print.
//!
//! The point a reader most often gets wrong: a line that does not begin
//! with `/` is work for the selected room rather than an unknown verb,
//! and the argument of a wire verb is the wire's own JSON — inventing a
//! second argument grammar here would describe every wire type twice.

use kernel::{Address, IdemKey};

pub(super) const CONTROL: [&str; 6] = ["help", "web", "at", "quit", "serving", "remote"];

/// The two Commands whose `Command::idem()` is `None`, which a socket
/// cannot carry: `PutSecret` has no wire form at all, and `Auth` is
/// proved in the handshake rather than sent as a command. Offering them
/// would list verbs that can only ever be refused.
const OFF_THE_SOCKET: [&str; 2] = ["PutSecret", "Auth"];

/// The Commands a console line can become, in wire order.
pub(super) fn carried_commands() -> impl Iterator<Item = &'static str> {
    wire::COMMAND_NAMES
        .iter()
        .copied()
        .filter(|name| !OFF_THE_SOCKET.contains(name))
}

/// What one line asked for.
#[derive(Debug, PartialEq)]
pub(crate) enum Line {
    /// An empty line, which is not a question.
    Nothing,
    Help,
    OpenWeb,
    /// What this process is doing: where it listens, what guards it, and
    /// the city's own counts beside it.
    Serving,
    Select(Address),
    Quit,
    /// The remote door: open it, pair a device, close it, list or
    /// revoke devices. Never a frame, so nothing on the wire can ask.
    Remote(crate::outside::console::RemoteLine),
    /// A wire verb, already built into the frame it names.
    Frame(Box<wire::ClientFrame>),
    /// Anything that does not begin with `/`: work for the selected
    /// room, in the words the person used.
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

/// Every verb this console answers to.
///
/// **A projection, never a second list.** The wire half is derived from
/// [`carried_commands`] and `wire::QUERY_NAMES`, so a command
/// renamed there is renamed here in the same commit or not at all. A
/// hand-written table would be a second vocabulary, and the moment it
/// drifted nothing would say so.
pub(crate) fn verbs() -> Vec<String> {
    let mut out: Vec<String> = CONTROL.iter().map(|name| (*name).to_owned()).collect();
    out.extend(carried_commands().map(snake));
    out.extend(wire::QUERY_NAMES.iter().map(|name| snake(name)));
    out
}

/// The verbs closest to something a person typed: those sharing the
/// longest prefix with it that any verb shares at all.
///
/// Cheap on purpose. An edit distance would be a better guess and a
/// worse answer, because somebody who mistyped a verb wants the short
/// list they can read, not the one winner they then have to doubt.
fn nearest(verb: &str) -> Vec<String> {
    let known = verbs();
    let typed: Vec<char> = verb.chars().collect();
    for length in (1..=typed.len().min(4)).rev() {
        let head: String = typed.iter().take(length).collect();
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

/// Reads one line; a wire Command whose body carries no `idem` gets
/// `idem`, the key this console minted for the line.
///
/// # Errors
/// None: an unreadable line is an answer (`Unknown`), because a console
/// that refused to classify a line would have nothing to say about it.
pub(crate) fn parse(line: &str, selected: Option<&Address>, idem: IdemKey) -> Line {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Line::Nothing;
    }
    let Some(rest) = trimmed.strip_prefix('/') else {
        // Plain text is work. Naming a room by hand for every task would
        // make the common case the expensive one.
        return match selected {
            Some(_) => Line::Work(trimmed.to_owned()),
            None => Line::Unknown {
                verb: "(no room selected)".to_owned(),
                nearest: vec!["at".to_owned()],
            },
        };
    };
    let (verb, tail) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
    let tail = tail.trim();
    match verb {
        "help" | "?" => Line::Help,
        "web" => Line::OpenWeb,
        "serving" => Line::Serving,
        "quit" | "exit" => Line::Quit,
        "remote" => Line::Remote(crate::outside::console::parse(tail)),
        "at" => match Address::parse(tail) {
            Ok(addr) => Line::Select(addr),
            Err(_) => Line::Unknown {
                verb: format!("at {tail}"),
                nearest: vec!["at <building>/<room>".to_owned()],
            },
        },
        other => wire_frame(other, tail, idem),
    }
}

/// A wire verb and the rest of the line, turned into the frame it names.
///
/// The body is JSON because the wire is JSON: inventing a second
/// argument grammar here would be a second description of every type on
/// the wire, and the two would disagree the first time either moved.
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
                verb: verb.to_owned(),
                nearest: nearest(verb),
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

/// What the console prints when asked what it knows.
///
/// Grouped the way the wire groups itself, and generated from the same
/// two constants the parser reads.
pub(crate) fn help(selected: Option<&Address>) -> String {
    let commands: Vec<String> = carried_commands().map(snake).collect();
    let queries: Vec<String> = wire::QUERY_NAMES.iter().map(|n| snake(n)).collect();
    let room = selected.map_or_else(
        || "no room selected - `/at <building>/<room>` first".to_owned(),
        |addr| format!("work goes to {}", addr.as_str()),
    );
    format!(
        "\n  {room}\n\n  \
         /help                     this\n  \
         /at <building>/<room>     choose where plain lines go\n  \
         /serving                  where this city listens, and what is running in it\n  \
         /web                      open the WebUI, token included\n  \
         /remote <verb>            open|pair|close|devices|revoke: the door a device reaches the city through\n  \
         /quit                     close this console; the city keeps serving\n\n  \
         anything else             work, dispatched to the chosen room\n\n  \
         /<query>                  {}\n  \
         /<command> <json>         {}\n",
        queries.join(", "),
        commands.join(", ")
    )
}

/// The ask frame for one question. The console answers in place, so
/// every question it asks carries the same number: nothing waits to be
/// paired.
fn asked(query: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "ask": { "ask_id": 0, "query": query } })
}
