// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a person pastes, read as one agent entry: a command line, a Zed
//! or JetBrains `agent_servers` block, or a registry `agent.json`
//! (`crates/agent_protocols/Spec.lean` §8-19).
//!
//! The grammar lives here and nowhere else, so the WebUI and the CLI read
//! a paste the same way (`Query::ParseAgentSpec`). Nothing is handed to a
//! shell: a command line is split on whitespace, a double-quoted stretch
//! is one word, and a shell's operators are refused rather than read.

use kernel::{AxCode, AxError};
use serde_json::Value;

use super::catalog::{pairs, registry_entry};
use super::entry::{AgentEntry, AgentId, AgentSource, Launch};

/// The characters a shell reads as plumbing.
const SHELL_OPERATORS: [char; 5] = ['|', '&', ';', '<', '>'];

/// Reads one pasted agent.
///
/// # Errors
/// `E_INVALID_ARGS` for empty text, a JSON block that is none of the two
/// shapes or names other than one agent, a command line with a shell
/// operator or an unclosed quote, and an id [`AgentId::parse`] refuses.
pub fn pasted(text: &str) -> Result<AgentEntry, AxError> {
    let text = text.trim();
    if text.starts_with('{') {
        let value: Value = serde_json::from_str(text)
            .map_err(|err| refused(&format!("the block is not JSON: {err}")))?;
        return block(&value);
    }
    command_line(text)
}

fn block(value: &Value) -> Result<AgentEntry, AxError> {
    if value.get("distribution").is_some() {
        return registry_entry(value, AgentSource::Pasted)?
            .ok_or_else(|| refused("the agent.json offers nothing this platform can start"));
    }
    let servers = value
        .get("agent_servers")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            refused("a JSON block is an `agent_servers` table or a registry agent.json")
        })?;
    let mut named = servers.iter();
    let (Some((name, server)), None) = (named.next(), named.next()) else {
        return Err(refused("paste one agent at a time"));
    };
    let program = server
        .get("command")
        .and_then(Value::as_str)
        .filter(|command| !command.trim().is_empty())
        .ok_or_else(|| refused(&format!("{name}: no `command`")))?;
    let args = server
        .get("args")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    Ok(AgentEntry {
        id: AgentId::parse(&id_of(name))?,
        name: name.clone(),
        source: AgentSource::Pasted,
        launch: Launch {
            program: program.to_owned(),
            args,
            env: pairs(server.get("env")),
        },
        version: None,
        licence: None,
    })
}

fn command_line(text: &str) -> Result<AgentEntry, AxError> {
    if text.contains(SHELL_OPERATORS) {
        return Err(refused(
            "a shell operator (`|`, `&`, `;`, `<`, `>`) is not run",
        ));
    }
    let mut words = words(text)?.into_iter();
    let program = words.next().ok_or_else(|| refused("nothing was pasted"))?;
    let name = std::path::Path::new(&program)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(program.as_str())
        .to_owned();
    Ok(AgentEntry {
        id: AgentId::parse(&id_of(&name))?,
        name,
        source: AgentSource::Pasted,
        launch: Launch {
            program,
            args: words.collect(),
            env: Vec::new(),
        },
        version: None,
        licence: None,
    })
}

/// Whitespace separates words; a double-quoted stretch is part of one
/// word and keeps its spaces; `\"` inside quotes is a quote.
fn words(text: &str) -> Result<Vec<String>, AxError> {
    let mut words = Vec::new();
    let mut word: Option<String> = None;
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match (quoted, c) {
            (true, '\\') if chars.peek() == Some(&'"') => {
                word.get_or_insert_with(String::new).push('"');
                chars.next();
            }
            (_, '"') => {
                quoted = !quoted;
                word.get_or_insert_with(String::new);
            }
            (false, c) if c.is_whitespace() => words.extend(word.take()),
            (_, c) => word.get_or_insert_with(String::new).push(c),
        }
    }
    if quoted {
        return Err(refused("a double quote is not closed"));
    }
    words.extend(word);
    Ok(words)
}

/// An id from a name: lowercase, every character outside the id grammar
/// turned into `-`.
fn id_of(name: &str) -> String {
    name.trim()
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => c.to_ascii_lowercase(),
            _ => '-',
        })
        .collect::<String>()
        .trim_start_matches(['-', '_', '.'])
        .to_owned()
}

fn refused(why: &str) -> AxError {
    AxError::failure(AxCode::InvalidArgs, "read a pasted agent", why.to_owned()).with_recovery(
        "paste one program and its arguments, a Zed or JetBrains `agent_servers` block with one \
         agent, or a registry agent.json",
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn a_quoted_path_with_spaces_is_one_word_and_operators_are_refused() {
        let entry = pasted(r#""C:\Program Files\agent\agent.exe" --acp "say \"hi\"""#).unwrap();
        assert_eq!(
            entry.launch,
            Launch {
                program: r"C:\Program Files\agent\agent.exe".to_owned(),
                args: vec!["--acp".to_owned(), "say \"hi\"".to_owned()],
                env: Vec::new(),
            }
        );
        assert_eq!(entry.id.as_str(), "agent");
        assert_eq!(
            pasted("npx -y foo@1 && rm -rf ~").unwrap_err().code(),
            &AxCode::InvalidArgs
        );
    }

    #[test]
    fn an_agent_servers_block_names_exactly_one_agent() {
        let one = r#"{"agent_servers":{"My Agent":{"command":"my-agent","args":["acp"],"env":{"MODE":"x"}}}}"#;
        let entry = pasted(one).unwrap();
        assert_eq!(
            (entry.id.as_str(), entry.launch.preview(), entry.launch.env),
            (
                "my-agent",
                "my-agent acp".to_owned(),
                vec![("MODE".to_owned(), "x".to_owned())]
            )
        );
        let two = r#"{"agent_servers":{"a":{"command":"a"},"b":{"command":"b"}}}"#;
        assert!(pasted(two).is_err());
    }
}
