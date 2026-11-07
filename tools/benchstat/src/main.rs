// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Command-line boundary for benchmark analysis.

use std::process::ExitCode;

use benchstat::error::Severity;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(round) = args.get(1) else {
        eprintln!("E_USAGE parse command line: missing round directory; use `benchstat export|verify|report <round-dir> [--write]`");
        return ExitCode::from(2);
    };
    let result = match args.first().map(String::as_str) {
        Some("export") => benchstat::export(std::path::Path::new(round)),
        Some("verify") => benchstat::verify(std::path::Path::new(round)),
        _ => {
            eprintln!("E_USAGE parse command line: expected export, verify or report; use `benchstat export|verify|report <round-dir> [--write]`");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(computed) => {
            print!("{}", computed.exports);
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
