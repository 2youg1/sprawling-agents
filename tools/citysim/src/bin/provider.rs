// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The stand-in provider as a process (citysim-SPEC.md 8-10):
//! `provider <script.json> <record.jsonl> [<listen>]`.
//!
//! It binds a loopback port, prints `SPRAWLING_PROVIDER=<url>` as its
//! first line so the recipe that started it can hand the URL on, and
//! then plays the script until it is stopped. It writes no text of its
//! own into a reply: every reply is a line of the script.

use std::convert::Infallible;
use std::io::Write as _;
use std::net::{SocketAddr, TcpListener};
use std::path::Path;
use std::process::ExitCode;

use citysim::{ScriptedProvider, WireScript};
use kernel::{AxCode, AxError};

/// Where the provider listens when the command names nowhere: any free
/// port on this machine's loopback address.
const LISTEN: &str = "127.0.0.1:0";

/// The variable the printed line assigns, which the recipe exports to
/// the check it starts.
const VARIABLE: &str = "SPRAWLING_PROVIDER";

fn main() -> ExitCode {
    match serve() {
        Ok(never) => match never {},
        Err(err) => {
            eprintln!("provider failed: {err}");
            eprintln!("recovery: {}", err.recovery());
            ExitCode::FAILURE
        }
    }
}

/// Plays the script until the record cannot be written; a connection
/// that fails ends that connection only.
fn serve() -> Result<Infallible, AxError> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (script, record, listen) = match args.as_slice() {
        [script, record] => (script, record, LISTEN),
        [script, record, listen] => (script, record, listen.as_str()),
        _ => return Err(invalid("read the command line", &args.join(" "))),
    };
    let text = std::fs::read_to_string(script).map_err(|err| {
        AxError::failure(
            AxCode::ConfigInvalid,
            "read a provider script",
            format!("{script}: {err}"),
        )
        .with_recovery("name a script file this process can read")
    })?;
    let listen: SocketAddr = listen
        .parse()
        .map_err(|_| invalid("read the listen address", listen))?;
    if !listen.ip().is_loopback() {
        return Err(invalid("listen on a loopback address", &listen.to_string()));
    }
    let listener = TcpListener::bind(listen).map_err(|err| {
        AxError::failure(
            AxCode::ToolUnavailable,
            "bind the provider's port",
            format!("{listen}: {err}"),
        )
        .with_recovery("name a free port, or leave the address out for any free one")
    })?;
    let mut provider =
        ScriptedProvider::open(listener, WireScript::parse(&text)?, Path::new(record))?;
    println!("{VARIABLE}={}", provider.url()?);
    std::io::stdout().flush().map_err(|err| {
        AxError::failure(
            AxCode::ToolUnavailable,
            "print the provider's URL",
            err.to_string(),
        )
        .with_recovery("start the provider with a standard output something reads")
    })?;
    loop {
        match provider.answer_one() {
            Ok(()) => {}
            Err(err) if err.code() == &AxCode::StorageFatal => return Err(err),
            Err(err) => eprintln!("provider: {err}"),
        }
    }
}

fn invalid(action: &str, subject: &str) -> AxError {
    AxError::failure(AxCode::InvalidArgs, action.to_owned(), subject.to_owned()).with_recovery(
        "run `provider <script.json> <record.jsonl> [<listen>]`, where listen is a loopback \
         address such as 127.0.0.1:0",
    )
}
