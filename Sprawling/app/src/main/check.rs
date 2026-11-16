// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;
use std::process::ExitCode;

use super::city::report;

pub(super) fn verb(city: Option<&String>) -> ExitCode {
    let Some(city) = city else {
        eprintln!("usage: sprawling check <city>");
        return ExitCode::from(2);
    };
    let root = Path::new(city);
    match city::check(root) {
        Ok(checked) if checked.findings.is_empty() => {
            println!("ok: {} file(s)", checked.read);
            ExitCode::SUCCESS
        }
        Ok(checked) => {
            for finding in &checked.findings {
                eprintln!("{}", line(root, finding));
            }
            ExitCode::FAILURE
        }
        Err(err) => report(err),
    }
}

fn line(root: &Path, finding: &city::Finding) -> String {
    let path = finding
        .path
        .strip_prefix(root)
        .unwrap_or(&finding.path)
        .to_string_lossy()
        .replace('\\', "/");
    let place = match finding.at {
        Some(city::Position { line, column }) => format!("{path}:{line}:{column}"),
        None => path,
    };
    let error = &finding.error;
    format!("{place}: {}: {}", error.code().as_str(), error.subject())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_located_refusal_reads_as_a_jump_target_relative_to_the_city() {
        let root = Path::new("city");
        let finding = city::Finding {
            path: root.join(".sprawling").join("CONFIG.toml"),
            at: Some(city::Position { line: 2, column: 3 }),
            error: kernel::AxError::failure(kernel::AxCode::ConfigInvalid, "read", "wrong")
                .with_recovery("fix it"),
        };

        assert_eq!(
            line(root, &finding),
            format!(
                ".sprawling/CONFIG.toml:2:3: {}: wrong",
                kernel::AxCode::ConfigInvalid.as_str()
            )
        );
    }
}
