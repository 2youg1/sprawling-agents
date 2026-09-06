// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a line typed into a serving city means (sprawling-SPEC.md
//! section 8-11).
//!
//! `sprawling up` used to print four lines and block until Ctrl-C. That
//! terminal is a surface the product threw away, and on a machine with
//! no browser it is the only surface there is.
//!
//! Everything here is a pure judgement over one line of text. What the
//! judgement produces is either a control action the terminal carries
//! out or a `ClientFrame` that goes to the same desk a browser's frames
//! go to, so the console decides nothing the server does not.

use kernel::Address;

pub(super) const CONTROL: [&str; 5] = ["help", "web", "at", "quit", "serving"];

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
    /// A wire verb, already built into the frame it names.
    Frame(Box<channels::ClientFrame>),
    /// Anything that does not begin with `/`: work for the selected
    /// room, in the words the person used.
    Work(String),
    /// A verb this console does not have, with the nearest ones it does.
    Unknown {
        verb: String,
        nearest: Vec<String>,
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
/// `channels::COMMAND_NAMES` and `channels::QUERY_NAMES`, so a command
/// renamed there is renamed here in the same commit or not at all. A
/// hand-written table would be a second vocabulary, and the moment it
/// drifted nothing would say so.
pub(crate) fn verbs() -> Vec<String> {
    let mut out: Vec<String> = CONTROL.iter().map(|name| (*name).to_owned()).collect();
    out.extend(channels::COMMAND_NAMES.iter().map(|name| snake(name)));
    out.extend(channels::QUERY_NAMES.iter().map(|name| snake(name)));
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

/// Reads one line.
///
/// # Errors
/// None: an unreadable line is an answer (`Unknown`), because a console
/// that refused to classify a line would have nothing to say about it.
pub(crate) fn parse(line: &str, selected: Option<&Address>) -> Line {
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
        "at" => match Address::parse(tail) {
            Ok(addr) => Line::Select(addr),
            Err(_) => Line::Unknown {
                verb: format!("at {tail}"),
                nearest: vec!["at <building>/<room>".to_owned()],
            },
        },
        other => wire_frame(other, tail),
    }
}

/// A wire verb and the rest of the line, turned into the frame it names.
///
/// The body is JSON because the wire is JSON: inventing a second
/// argument grammar here would be a second description of every type on
/// the wire, and the two would disagree the first time either moved.
fn wire_frame(verb: &str, tail: &str) -> Line {
    let body = if tail.is_empty() { "null" } else { tail };
    let known_command = channels::COMMAND_NAMES
        .iter()
        .find(|name| snake(name) == verb);
    let known_query = channels::QUERY_NAMES
        .iter()
        .find(|name| snake(name) == verb);
    let framed = match (known_command, known_query) {
        (Some(name), _) => format!("{{\"command\":{{{}:{body}}}}}", quoted(name)),
        (None, Some(name)) => {
            if tail.is_empty() {
                format!("{{\"query\":{}}}", quoted(name))
            } else {
                format!("{{\"query\":{{{}:{body}}}}}", quoted(name))
            }
        }
        (None, None) => {
            return Line::Unknown {
                verb: verb.to_owned(),
                nearest: nearest(verb),
            };
        }
    };
    match serde_json::from_str::<channels::ClientFrame>(&framed) {
        Ok(frame) => Line::Frame(Box::new(frame)),
        Err(_) => Line::Unknown {
            verb: format!("{verb} {tail}"),
            nearest: vec![format!("{verb} takes a JSON body; see `sprawling call`")],
        },
    }
}

/// A wire name in its snake_case spelling, as a JSON key.
fn quoted(camel: &str) -> String {
    serde_json::Value::String(snake(camel)).to_string()
}

/// What the console prints when asked what it knows.
///
/// Grouped the way the wire groups itself, and generated from the same
/// two constants the parser reads.
pub(crate) fn help(selected: Option<&Address>) -> String {
    let commands: Vec<String> = channels::COMMAND_NAMES.iter().map(|n| snake(n)).collect();
    let queries: Vec<String> = channels::QUERY_NAMES.iter().map(|n| snake(n)).collect();
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
         /quit                     close this console; the city stops\n\n  \
         anything else             work, dispatched to the chosen room\n\n  \
         /<query>                  {}\n  \
         /<command> <json>         {}\n",
        queries.join(", "),
        commands.join(", ")
    )
}
