// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::exit::Exit;
use super::refusal::{Form, written};
use super::router::flag_value;
use super::verbs::{self, Verb};
use super::wire_client::{self, Listen, Spoken, Unheard, Until};
use kernel::consts_policy::DEFAULT_AT;

pub(super) fn call(args: &[String]) -> Exit {
    let Some(frame) = args.get(1).filter(|a| !a.starts_with("--")) else {
        if let Some(row) = verbs::row(Verb::Call) {
            eprintln!("usage: {}", verbs::usage(row));
        }
        eprintln!(
            "exit: 0 answered, 1 refused, 2 this command line, 3 nothing came back, 4 no city at --at"
        );
        eprintln!("commands: {}", wire::COMMAND_NAMES.join(", "));
        eprintln!("queries:  {}", wire::QUERY_NAMES.join(", "));
        return Exit::Line;
    };

    let frame = if frame == "-" {
        match std::io::read_to_string(std::io::stdin()) {
            Ok(text) => text,
            Err(err) => {
                let refusal = kernel::AxError::failure(
                    kernel::AxCode::WireMismatch,
                    "read the frame to send from stdin",
                    err.to_string(),
                )
                .with_recovery("pipe one UTF-8 JSON frame into `sprawling call -`, or pass the frame as the argument");
                eprint!("{}", written(&refusal, Form::of(args)));
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
            let spoken = heard.spoken();
            match spoken {
                Spoken::Refused | Spoken::Answered => {}
                Spoken::Quiet | Spoken::Unfinished => {
                    eprintln!(
                        "what this call waits for did not come inside {quiet}ms: the city may still be working on it"
                    );
                    eprintln!(
                        "recovery: ask again with a longer --quiet-ms, or read the city's own log"
                    );
                }
            }
            exit_of(&spoken)
        }
        Err(unheard) => tell_unheard(&unheard, Form::of(args)),
    }
}

pub(super) fn exit_of(spoken: &Spoken) -> Exit {
    match spoken {
        Spoken::Refused => Exit::Refused,
        Spoken::Answered => Exit::Done,
        Spoken::Quiet | Spoken::Unfinished => Exit::Quiet,
    }
}

pub(super) fn tell_unheard(unheard: &Unheard, form: Form) -> Exit {
    let (exit, err) = match unheard {
        Unheard::Unreadable(err) | Unheard::Unkeyed(err) => (Exit::Line, err),
        Unheard::NoCity(err) => (Exit::NoCity, err),
        Unheard::Broken(err) => (Exit::Refused, err),
    };
    eprint!("{}", written(err, form));
    exit
}
