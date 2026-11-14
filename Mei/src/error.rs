// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Stable, actionable failures, with raw Provider bodies kept separate
//! (`Mei/Spec.lean` §8 and §12).

use std::fmt::{Display, Formatter};

/// The result used by every Mei adapter.
pub type Result<T> = std::result::Result<T, Error>;

/// The stable code naming the rule a diagnostic enforces. One variant per
/// rule, so a misspelled or newly added code fails at compile time instead
/// of landing in a catch-all arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    Usage,
    Config,
    Transport,
    Protocol,
    Python,
    Unsupported,
}

impl Code {
    /// The wire spelling, stable across releases.
    pub fn as_str(self) -> &'static str {
        match self {
            Code::Usage => "E_MEI_USAGE",
            Code::Config => "E_MEI_CONFIG",
            Code::Transport => "E_MEI_TRANSPORT",
            Code::Protocol => "E_MEI_PROTOCOL",
            Code::Python => "E_MEI_PYTHON",
            Code::Unsupported => "E_MEI_UNSUPPORTED",
        }
    }

    /// The rule this code enforces; one arm per variant, no default.
    fn rule(self) -> &'static str {
        match self {
            Code::Usage => "Only `mei -p PROMPT` or a non-terminal stdin prompt is accepted",
            Code::Config => {
                "Configuration must preserve the closed Messages and transport contract"
            }
            Code::Transport => {
                "Only plain HTTP with a complete status line, header/body boundary and JSON body is accepted"
            }
            Code::Protocol => {
                "The response must satisfy the minimal Anthropic Messages and sole-Python-tool contract"
            }
            Code::Python => {
                "The private Python channel must carry one valid JSON string frame per call and the interpreter must stay alive"
            }
            Code::Unsupported => "Mei requires POSIX process and signal semantics",
        }
    }
}

/// A failure with a stable code and a checked next step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: Code,
    pub action: Box<str>,
    pub subject: Box<str>,
    pub path: Option<Box<str>>,
    pub value: Option<Box<str>>,
    pub recovery: Box<str>,
}

impl Diagnostic {
    /// One diagnostic; its rule is derived from the code in one place.
    pub fn new(
        code: Code,
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
        self.code.rule()
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
        writeln!(out, "code = {:?}", self.code.as_str())?;
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

/// Convenience for the frequent protocol family: a `Diagnostic` carrying
/// the `Protocol` code, ready for `with_path`/`with_value`. `?` and
/// [`From<Diagnostic>`] lift it into [`Error`] at the call site.
pub fn protocol(
    action: impl Into<String>,
    subject: impl Into<String>,
    recovery: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(Code::Protocol, action, subject, recovery)
}

impl From<Diagnostic> for Error {
    fn from(diagnostic: Diagnostic) -> Error {
        Error::Diagnostic(diagnostic)
    }
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

    #[test]
    fn diagnostic_values_use_toml_string_escapes() {
        let error = protocol(
            "read response.content",
            "content is invalid",
            "return valid content",
        )
        .with_value("quote \" \\ \t \u{1b} \0 \u{7f}");
        let shown = error.to_string();
        assert!(
            shown.contains(r#"value = "quote \" \\ \t \u001B \u0000 \u007F""#),
            "{shown}"
        );
    }
}
