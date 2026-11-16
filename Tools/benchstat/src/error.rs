// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Rejected,

    Broken,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code} {action}: {subject}; {recovery}")]
pub struct BenchError {
    pub code: &'static str,

    pub action: String,

    pub subject: String,

    pub recovery: String,

    pub severity: Severity,
}

impl BenchError {
    pub(crate) fn input(action: &str, subject: String, recovery: &str) -> Self {
        Self::new(
            "E_INVALID_INPUT",
            action,
            subject,
            Recovery {
                text: recovery.to_owned(),
                severity: Severity::Rejected,
            },
        )
    }

    pub(crate) fn mismatch(subject: String) -> Self {
        Self::new(
            "E_VERIFY_MISMATCH",
            "verify benchmark round",
            subject,
            Recovery {
                text: "regenerate the round with `benchstat export <round> --write`, then review the diff".to_owned(),
                severity: Severity::Rejected,
            },
        )
    }

    pub fn usage(subject: String) -> Self {
        Self::new(
            "E_USAGE",
            "parse command line",
            subject,
            Recovery {
                text: "use `benchstat export|verify|report <round-dir> [--write]`".to_owned(),
                severity: Severity::Broken,
            },
        )
    }

    pub(crate) fn io(action: &str, subject: String, source: std::io::Error) -> Self {
        Self::new(
            "E_IO",
            action,
            format!("{subject}: {source}"),
            Recovery {
                text: "check the path and its permissions, then retry".to_owned(),
                severity: Severity::Broken,
            },
        )
    }

    pub(crate) fn numeric(action: &str, subject: String, recovery: &str) -> Self {
        Self::new(
            "E_NUMERIC",
            action,
            subject,
            Recovery {
                text: recovery.to_owned(),
                severity: Severity::Broken,
            },
        )
    }

    fn new(code: &'static str, action: &str, subject: String, recovery: Recovery) -> Self {
        Self {
            code,
            action: action.to_owned(),
            subject,
            recovery: recovery.text,
            severity: recovery.severity,
        }
    }
}

struct Recovery {
    text: String,
    severity: Severity,
}
