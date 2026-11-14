// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Stable, actionable failures, with raw Provider bodies kept separate
//! (`Mei/Spec.lean` §8 and §12).

use std::fmt::{Display, Formatter};

/// The result used by every Mei adapter.
pub type Result<T> = std::result::Result<T, Error>;

/// A failure with a stable code and a checked next step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub action: Box<str>,
    pub subject: Box<str>,
    pub path: Option<Box<str>>,
    pub value: Option<Box<str>>,
    pub recovery: Box<str>,
}

impl Diagnostic {
    /// One diagnostic; its rule is derived from the stable code in one place.
    pub fn new(
        code: &'static str,
        action: impl Into<String>,
        subject: impl Into<String>,
        recovery: impl Into<String>,
    ) -> Diagnostic {
        Diagnostic {
            code,
            action: action.into().into_boxed_str(),
            subject: subject.into().into_boxed_str(),
            path: None,
            value: None,
            recovery: recovery.into().into_boxed_str(),
        }
    }

    /// Adds the offending value, when naming it helps the repair.
    pub fn with_value(mut self, value: impl Into<String>) -> Diagnostic {
        self.value = Some(value.into().into_boxed_str());
        self
    }

    /// Adds the full path of what failed, when one exists.
    pub fn with_path(mut self, path: impl Into<String>) -> Diagnostic {
        self.path = Some(path.into().into_boxed_str());
        self
    }

    fn rule(&self) -> &'static str {
        match self.code {
            "E_MEI_USAGE" => "Only `mei -p PROMPT` or a non-terminal stdin prompt is accepted",
            "E_MEI_CONFIG" => {
                "Configuration must preserve the closed Messages and transport contract"
            }
            "E_MEI_TRANSPORT" => {
                "Only plain HTTP with a complete status line, header/body boundary and JSON body is accepted"
            }
            "E_MEI_PROTOCOL" => {
                "The response must satisfy the minimal Anthropic Messages and sole-Python-tool contract"
            }
            "E_MEI_PYTHON" => {
                "The private Python channel must carry one valid JSON string frame per call and the interpreter must stay alive"
            }
            "E_MEI_UNSUPPORTED" => "Mei requires POSIX process and signal semantics",
            _ => "Correct the named operation before running Mei again",
        }
    }
}

/// A structured diagnostic or an unwrapped non-200 response body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Diagnostic(Diagnostic),
    Body(String),
}

impl Display for Diagnostic {
    fn fmt(&self, out: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(out, "```toml")?;
        writeln!(out, "code = {:?}", self.code)?;
        writeln!(out, "action = {:?}", self.action)?;
        writeln!(out, "subject = {:?}", self.subject)?;
        if let Some(path) = &self.path {
            writeln!(out, "path = {:?}", path)?;
        }
        if let Some(value) = &self.value {
            writeln!(out, "value = {:?}", value)?;
        }
        writeln!(out, "rule = {:?}", self.rule())?;
        writeln!(out, "recovery = {:?}", self.recovery)?;
        write!(out, "```")
    }
}

impl Display for Error {
    fn fmt(&self, out: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Diagnostic(diagnostic) => diagnostic.fmt(out),
            Error::Body(body) => out.write_str(body),
        }
    }
}

impl std::error::Error for Error {}

/// Convenience for the frequent protocol family.
pub fn protocol(
    action: impl Into<String>,
    subject: impl Into<String>,
    recovery: impl Into<String>,
) -> Error {
    Error::Diagnostic(Diagnostic::new("E_MEI_PROTOCOL", action, subject, recovery))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_diagnostic_is_one_actionable_toml_block() {
        let error = protocol(
            "read response.content",
            "content is missing",
            "return a Messages JSON object with content",
        );
        let shown = error.to_string();
        assert!(shown.starts_with("```toml\ncode = \"E_MEI_PROTOCOL\""));
        for field in ["action", "subject", "rule", "recovery"] {
            assert!(shown.contains(field), "{shown}");
        }
        assert!(shown.ends_with("```"));
    }
}
