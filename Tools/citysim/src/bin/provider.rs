// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::convert::Infallible;
use std::io::Write as _;
use std::net::{SocketAddr, TcpListener};
use std::path::Path;
use std::process::ExitCode;

use citysim::ScriptedProvider;
use kernel::{AxCode, AxError};

const LISTEN: &str = "127.0.0.1:0";

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

fn serve() -> Result<Infallible, AxError> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (script, record, listen) = match args.as_slice() {
        [script, record] => (script, record, LISTEN),
        [script, record, listen] => (script, record, listen.as_str()),
        _ => return Err(invalid("read the command line", &args.join(" "))),
    };
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
    let mut provider = ScriptedProvider::open(listener, Path::new(script), Path::new(record))?;
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
