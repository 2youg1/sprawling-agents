// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::fmt::{Display, Formatter};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    Usage,
    Capture,
    Config,
    Doctor,
    Transport,
    Protocol,
    Python,
    Session,
    Workspace,
    Unsupported,
}

impl Code {
    pub fn as_str(self) -> &'static str {
        match self {
            Code::Usage => "E_MEI_USAGE",
            Code::Capture => "E_MEI_CAPTURE",
            Code::Config => "E_MEI_CONFIG",
            Code::Doctor => "E_MEI_DOCTOR",
            Code::Transport => "E_MEI_TRANSPORT",
            Code::Protocol => "E_MEI_PROTOCOL",
            Code::Python => "E_MEI_PYTHON",
            Code::Session => "E_MEI_SESSION",
            Code::Workspace => "E_MEI_WORKSPACE",
            Code::Unsupported => "E_MEI_UNSUPPORTED",
        }
    }

    fn rule(self) -> &'static str {
        match self {
            Code::Usage => {
                "Only `mei [-p PROMPT] [--resume JSONL]` or a non-terminal stdin prompt is accepted"
            }
            Code::Capture => {
                "A capture directory must be new and run-owned; spilled files outlive their interpreter"
            }
            Code::Config => {
                "Configuration must preserve the closed Messages and transport contract"
            }
            Code::Doctor => {
                "A current-version cache or a working Python and uv environment is required; installation never updates Mei"
            }
            Code::Transport => {
                "Only plain HTTP with a complete status line, header/body boundary and JSON body is accepted"
            }
            Code::Protocol => {
                "The response must satisfy the minimal Anthropic Messages and sole-python-tool contract"
            }
            Code::Python => {
                "The private Python channel must carry one valid {capture, images, failed} response frame per call and the interpreter must stay alive"
            }
            Code::Session => {
                "A resume log must contain complete Mei messages from the initial user prompt through an assistant end_turn"
            }
            Code::Workspace => {
                "Only a readable UTF-8 AGENTS.md at the workspace's top level can open a new session"
            }
            Code::Unsupported => "This Mei build does not support the current platform",
        }
    }
}

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

    pub fn with_value(mut self, value: impl Into<String>) -> Diagnostic {
        self.value = Some(value.into().into_boxed_str());
        self
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Diagnostic {
        self.path = Some(path.into().into_boxed_str());
        self
    }

    fn rule(&self) -> &'static str {
        self.code.rule()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Diagnostic(Diagnostic),
    Body(String),
}

fn toml_string(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len().saturating_add(2));
    encoded.push('"');
    for ch in text.chars() {
        match ch {
            '"' => encoded.push_str("\\\""),
            '\\' => encoded.push_str("\\\\"),
            '\u{08}' => encoded.push_str("\\b"),
            '\t' => encoded.push_str("\\t"),
            '\n' => encoded.push_str("\\n"),
            '\u{0c}' => encoded.push_str("\\f"),
            '\r' => encoded.push_str("\\r"),
            '\u{00}'..='\u{1f}' | '\u{7f}' => {
                encoded.push_str(&format!("\\u{:04X}", u32::from(ch)));
            }
            _ => encoded.push(ch),
        }
    }
    encoded.push('"');
    encoded
}

fn toml_resolution(text: &str) -> String {
    let controls = text
        .chars()
        .any(|ch| ch.is_control() && ch != '\n' && ch != '\t');
    if text.contains("'''") || controls {
        toml_string(text)
    } else {
        format!("'''\n{text}\n'''")
    }
}

impl Display for Diagnostic {
    fn fmt(&self, out: &mut Formatter<'_>) -> std::fmt::Result {
        let description = format!("{}: {}", self.action, self.subject);
        let mut cause = self.rule().to_owned();
        if let Some(path) = &self.path {
            cause.push_str("\npath = ");
            cause.push_str(path);
        }
        if let Some(value) = &self.value {
            cause.push_str("\nvalue = ");
            cause.push_str(value);
        }
        writeln!(out, "```toml")?;
        writeln!(out, "code = {}", toml_string(self.code.as_str()))?;
        writeln!(out, "description = {}", toml_string(&description))?;
        writeln!(out, "cause = {}", toml_string(&cause))?;
        writeln!(out, "resolution = {}", toml_resolution(&self.recovery))?;
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

pub fn remove_file_command(path: &std::path::Path) -> Option<String> {
    removal(path, "-f", "")
}

pub fn remove_tree_command(path: &std::path::Path) -> Option<String> {
    removal(path, "-rf", " -Recurse")
}

pub fn remove_file_instruction(path: &std::path::Path) -> String {
    remove_file_command(path).unwrap_or_else(|| {
        "delete the file at the native path named in `path` with your file manager; no lossy shell command is offered".to_owned()
    })
}

pub fn remove_tree_instruction(path: &std::path::Path) -> String {
    remove_tree_command(path).unwrap_or_else(|| {
        "delete the tree at the native path named in `path` with your file manager; no lossy shell command is offered".to_owned()
    })
}

pub fn move_guide_to_backup_command(path: &std::path::Path) -> Option<String> {
    let destination = path.with_file_name("AGENTS.md.not-a-guide");
    let shown = path.to_str()?;
    let destination = destination.to_str()?;
    if cfg!(windows) {
        Some(format!(
            "if (Test-Path -LiteralPath '{}') {{ throw 'the backup already exists' }}; Move-Item -LiteralPath '{}' -Destination '{}'",
            powershell_quote(destination),
            powershell_quote(shown),
            powershell_quote(destination)
        ))
    } else {
        Some(format!(
            "test ! -e '{}' && mv -- '{}' '{}'",
            unix_quote(destination),
            unix_quote(shown),
            unix_quote(destination)
        ))
    }
}

fn removal(path: &std::path::Path, rm_flag: &str, recurse: &str) -> Option<String> {
    let shown = path.to_str()?;
    if cfg!(windows) {
        Some(format!(
            "Remove-Item -LiteralPath '{}'{} -Force",
            powershell_quote(shown),
            recurse
        ))
    } else {
        Some(format!("rm {rm_flag} -- '{}'", unix_quote(shown)))
    }
}

fn powershell_quote(text: &str) -> String {
    text.replace('\'', "''")
        .replace('‘', "‘‘")
        .replace('’', "’’")
}

fn unix_quote(text: &str) -> String {
    text.replace('\'', "'\\''")
}

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
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::disallowed_methods,
        reason = "test code"
    )]

    use super::*;

    #[test]
    fn a_diagnostic_is_description_cause_resolution() {
        let error = protocol(
            "read response.content",
            "content is missing",
            "return a Messages JSON object with content",
        );
        let shown = error.to_string();
        assert!(shown.starts_with("```toml\ncode = \"E_MEI_PROTOCOL\""));
        for field in ["description", "cause", "resolution"] {
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
            shown.contains(r#"quote \" \\ \t \u001B \u0000 \u007F"#),
            "{shown}"
        );
    }

    #[test]
    fn removal_commands_quote_the_platform_shell() {
        let path = std::path::Path::new("a b/it's.txt");
        let file = remove_file_command(path).unwrap();
        let tree = remove_tree_command(path).unwrap();
        if cfg!(windows) {
            assert_eq!(file, "Remove-Item -LiteralPath 'a b/it''s.txt' -Force");
            assert_eq!(
                tree,
                "Remove-Item -LiteralPath 'a b/it''s.txt' -Recurse -Force"
            );
        } else {
            assert_eq!(file, "rm -f -- 'a b/it'\\''s.txt'");
            assert_eq!(tree, "rm -rf -- 'a b/it'\\''s.txt'");
        }
    }

    #[test]
    fn the_displayed_resolution_is_the_command_verbatim() {
        let recovery = if cfg!(windows) {
            remove_file_command(std::path::Path::new("O’Brien/it's.txt")).unwrap()
        } else {
            remove_file_command(std::path::Path::new("work space/it's.txt")).unwrap()
        };
        let shown = protocol("read a guide", "it is unreadable", &recovery).to_string();
        let body = shown
            .split("resolution = '''\n")
            .nth(1)
            .and_then(|rest| rest.split("\n'''\n```").next())
            .unwrap();
        assert_eq!(body, recovery);
    }

    #[cfg(unix)]
    #[test]
    fn removal_stops_option_parsing_and_refuses_lossy_paths() {
        assert!(
            remove_file_command(std::path::Path::new("-tools/uv"))
                .unwrap()
                .starts_with("rm -f -- ")
        );
        use std::os::unix::ffi::OsStringExt as _;
        let path = std::path::PathBuf::from(std::ffi::OsString::from_vec(vec![0xff]));
        assert_eq!(remove_file_command(&path), None);
        assert!(!remove_file_instruction(&path).contains("rm -f"));
    }

    #[test]
    fn the_rename_repair_preserves_both_directories() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("AGENTS.md");
        let backup = directory.path().join("AGENTS.md.not-a-guide");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("keep"), "keep").unwrap();
        let command = move_guide_to_backup_command(&source).unwrap();
        let run = || {
            if cfg!(windows) {
                std::process::Command::new("powershell")
                    .args(["-NoProfile", "-Command", &command])
                    .status()
                    .unwrap()
            } else {
                std::process::Command::new("sh")
                    .args(["-c", &command])
                    .status()
                    .unwrap()
            }
        };
        assert!(run().success());
        assert_eq!(
            std::fs::read_to_string(backup.join("keep")).unwrap(),
            "keep"
        );
        std::fs::create_dir(&source).unwrap();
        assert!(!run().success());
        assert!(source.exists());
        assert_eq!(
            std::fs::read_to_string(backup.join("keep")).unwrap(),
            "keep"
        );
    }

    #[cfg(windows)]
    #[test]
    fn powershell_restores_ascii_and_curly_quotes() {
        let path = "C:\\Work\\O’Brien\\it’s and ‘quoted’'s.txt";
        let command = format!(
            "[Console]::OutputEncoding = [Text.Encoding]::UTF8; [Console]::Write('{}')",
            powershell_quote(path)
        );
        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &command])
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap(), path);
    }
}
