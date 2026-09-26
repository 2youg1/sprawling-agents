// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `sprawling call`: one frame over the wire, and the exit code that
//! says what was observed (sprawling-SPEC.md 8-41, 8-91).

use super::exit::Exit;
use super::refusal::{Form, written};
use super::router::flag_value;
use super::wire_client::{self, Listen, Unheard, Until};
use kernel::consts_policy::DEFAULT_AT;

/// Sends one frame over the wire and prints everything that comes back.
///
/// An agent driving this learns the outcome from the exit code rather
/// than by parsing JSON, and the code says exactly what was observed:
/// 0 the city answered, 1 the city refused, 2 this command line or its
/// frame was not readable, 3 the city said nothing before the quiet
/// window closed, 4 nothing at `--at` answered as a city. Silence has
/// its own code because silence is not acceptance - the city may still
/// be working - and reading it as either of the first two is how a
/// failure becomes a success.
pub(super) fn call(args: &[String]) -> Exit {
    let Some(frame) = args.get(1).filter(|a| !a.starts_with("--")) else {
        eprintln!(
            "usage: sprawling call <frame-json|-> [--at host:port] [--token T] [--quiet-ms N] [--until <event-kind>] [--json]"
        );
        eprintln!(
            "exit: 0 answered, 1 refused, 2 this command line, 3 nothing came back, 4 no city at --at"
        );
        eprintln!("commands: {}", channels::COMMAND_NAMES.join(", "));
        eprintln!("queries:  {}", channels::QUERY_NAMES.join(", "));
        return Exit::Line;
    };
    // `-` reads the frame from stdin, which is how a frame too long for
    // one command line, or one a script generated, gets in.
    let frame = if frame == "-" {
        match std::io::read_to_string(std::io::stdin()) {
            Ok(text) => text,
            Err(err) => {
                eprintln!("could not read the frame from stdin: {err}");
                return Exit::Refused;
            }
        }
    } else {
        frame.clone()
    };
    let at = flag_value(args, "--at").unwrap_or_else(|| DEFAULT_AT.to_owned());
    let token = flag_value(args, "--token");
    let quiet = match flag_value(args, "--quiet-ms") {
        None => 2_000,
        Some(raw) => match raw.parse::<u64>() {
            Ok(ms) => ms,
            Err(_) => {
                eprintln!("not a number of milliseconds: {raw}");
                return Exit::Line;
            }
        },
    };
    // The kind is read through `EventKind`'s own serde spelling, so the
    // names this flag accepts are the names the wire prints.
    let until = match flag_value(args, "--until") {
        None => Until::Quiet,
        Some(raw) => match serde_json::from_value(serde_json::Value::String(raw.clone())) {
            Ok(kind) => Until::Event(kind),
            Err(_) => {
                eprintln!("not an event kind: {raw}");
                return Exit::Line;
            }
        },
    };
    let listen = Listen {
        quiet: std::time::Duration::from_millis(quiet),
        until,
    };
    match wire_client::call(&at, &frame, token.as_deref(), listen) {
        Ok(heard) => {
            eprintln!("{} frame(s), {} refusal(s)", heard.frames, heard.refusals);
            match heard.spoken() {
                wire_client::Spoken::Refused => Exit::Refused,
                wire_client::Spoken::Answered => Exit::Done,
                wire_client::Spoken::Quiet => {
                    eprintln!(
                        "what this call waits for did not come inside {quiet}ms: the city may still be working on it"
                    );
                    eprintln!(
                        "recovery: ask again with a longer --quiet-ms, or read the city's own log"
                    );
                    Exit::Quiet
                }
            }
        }
        Err(unheard) => {
            let (exit, err) = match &unheard {
                Unheard::Unreadable(err) => (Exit::Line, err),
                Unheard::NoCity(err) => (Exit::NoCity, err),
                Unheard::Broken(err) => (Exit::Refused, err),
            };
            eprint!("{}", written(err, Form::of(args)));
            exit
        }
    }
}
