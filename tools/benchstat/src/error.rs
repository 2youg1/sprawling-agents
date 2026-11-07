// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Stable command failures with an action, subject and recovery.

/// The process-level meaning of a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// The round or its published outputs are wrong.
    Rejected,
    /// The invocation or the analyzer's own environment failed.
    Broken,
}

/// One failure in the command's stable text format.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code} {action}: {subject}; {recovery}")]
pub struct BenchError {
    /// Stable machine-readable code.
    pub code: &'static str,
    /// The attempted action.
    pub action: String,
    /// The path, field or statistic being acted on.
    pub subject: String,
    /// The next action a caller can take.
    pub recovery: String,
    /// Which process exit this failure earns.
    pub severity: Severity,
}

impl BenchError {
    /// Reject malformed or inconsistent round input.
    pub(crate) fn input(action: &str, subject: String, recovery: &str) -> Self {
        Self::new("E_INVALID_INPUT", action, subject, recovery, Severity::Rejected)
    }

    /// Report a difference between published and recomputed artifacts.
    pub(crate) fn mismatch(subject: String) -> Self {
        Self::new(
            "E_VERIFY_MISMATCH",
            "verify benchmark round",
            subject,
            "regenerate the round with `benchstat export <round> --write`, then review the diff",
            Severity::Rejected,
        )
    }

    /// Reject a command line that names no valid operation.
    pub(crate) fn usage(subject: String) -> Self {
        Self::new(
            "E_USAGE",
            "parse command line",
            subject,
            "use `benchstat export|verify|report <round-dir> [--write]`",
            Severity::Broken,
        )
    }

    /// Report an operating-system read or write failure.
    pub(crate) fn io(action: &str, subject: String, source: std::io::Error) -> Self {
        Self::new(
            "E_IO",
            action,
            format!("{subject}: {source}"),
            "check the path and its permissions, then retry",
            Severity::Broken,
        )
    }

    /// Report an estimator that cannot produce a finite answer.
    pub(crate) fn numeric(action: &str, subject: String, recovery: &str) -> Self {
        Self::new("E_NUMERIC", action, subject, recovery, Severity::Broken)
    }

    fn new(
        code: &'static str,
        action: &str,
        subject: String,
        recovery: &str,
        severity: Severity,
    ) -> Self {
        Self {
            code,
            action: action.to_owned(),
            subject,
            recovery: recovery.to_owned(),
            severity,
        }
    }
}
