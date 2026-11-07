// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Command-line boundary for benchmark analysis.

use std::path::Path;
use std::process::ExitCode;

use benchstat::error::Severity;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            match error.severity {
                Severity::Rejected => ExitCode::FAILURE,
                Severity::Broken => ExitCode::from(2),
            }
        }
    }
}

fn run(args: &[String]) -> Result<String, benchstat::BenchError> {
    let command = args.first().map(String::as_str);
    let round = args
        .get(1)
        .ok_or_else(|| benchstat::BenchError::usage("missing round directory".to_owned()))?;
    if args.len() > 3 || args.get(2).is_some_and(|arg| arg != "--write") {
        return Err(benchstat::BenchError::usage(
            "unexpected arguments".to_owned(),
        ));
    }
    let write = args.get(2).is_some_and(|arg| arg == "--write");
    let round = Path::new(round);
    match command {
        Some("export") => {
            let computed = if write {
                benchstat::export_write(round)?
            } else {
                benchstat::export(round)?
            };
            if write {
                Ok("benchstat export: wrote exports.json and README.md\n".to_owned())
            } else {
                Ok(computed.exports)
            }
        }
        Some("verify") => {
            if write {
                return Err(benchstat::BenchError::usage(
                    "verify does not accept --write".to_owned(),
                ));
            }
            benchstat::verify(round)?;
            Ok("benchstat verify: exports and README reproduce byte for byte\n".to_owned())
        }
        Some("report") => {
            let computed = benchstat::report(round, write)?;
            if write {
                Ok("benchstat report: rewrote README.md from exports.json\n".to_owned())
            } else {
                Ok(computed.markdown)
            }
        }
        _ => Err(benchstat::BenchError::usage(
            "expected export, verify or report".to_owned(),
        )),
    }
}
