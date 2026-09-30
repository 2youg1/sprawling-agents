// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The `[[mcp]]` table: the servers one layer reaches, each read into
//! exactly one transport.

use kernel::{AxError, McpServer, McpTransport, ServerLabel};
use serde::Deserialize;

use super::refuse::refuse;

/// Reads one layer's `[[mcp]]` entries.
///
/// # Errors
/// Refuses a label `kernel` does not accept, a row that names both a
/// command and a url or neither, a stream chosen for a command, and two
/// rows under one label.
pub(super) fn servers(entries: Vec<McpSection>) -> Result<Vec<McpServer>, AxError> {
    let mut servers: Vec<McpServer> = Vec::new();
    for entry in entries {
        let label = ServerLabel::parse(&entry.label)
            .map_err(|err| refuse(format!("{}: {}", entry.label, err.recovery())))?;
        // One transport or the other, never both and never
        // neither: a row that names a command and a url is a
        // row whose reader has to guess which one was meant.
        let transport = match (entry.command.as_deref(), entry.url.as_deref()) {
            (Some(command), None) if !command.trim().is_empty() => {
                // A command answers on its own pipes, so a
                // row that also names a stream is a row
                // stating two transports.
                if entry.transport.is_some() {
                    return Err(refuse(format!(
                        "{}: a command is spoken to over its own pipes; `transport` \
                         chooses between `http` and `sse` for a url",
                        label.as_str()
                    )));
                }
                McpTransport::Stdio {
                    command: command.to_owned(),
                    args: entry.args,
                    env: listed(entry.env),
                }
            }
            (None, Some(url)) if !url.trim().is_empty() => {
                let url = url.to_owned();
                let headers = listed(entry.headers);
                // Absent means `http`: that is what a url
                // reached by posting one message is, and it
                // is what every row written before this key
                // existed meant.
                match entry.transport.unwrap_or(ReachedBy::Http) {
                    ReachedBy::Http => McpTransport::Http { url, headers },
                    ReachedBy::Sse => McpTransport::Sse { url, headers },
                }
            }
            (Some(_), Some(_)) => {
                return Err(refuse(format!(
                    "{}: a server is reached by a command or by a url, not both",
                    label.as_str()
                )));
            }
            _ => {
                return Err(refuse(format!(
                    "{}: an mcp server needs a command to start or a url to reach",
                    label.as_str()
                )));
            }
        };
        // Two servers under one label would put one tool name
        // in front of two processes, and that is a routing
        // mistake rather than a preference.
        if servers.iter().any(|held| held.label == label) {
            return Err(refuse(format!(
                "{}: two servers are named the same in one layer",
                label.as_str()
            )));
        }
        servers.push(McpServer { label, transport });
    }
    Ok(servers)
}

/// A configured table as the wire carries it: name before value, in the
/// order a `BTreeMap` reads them, so two runs of the same file hand the
/// same list to the same server.
fn listed(table: std::collections::BTreeMap<String, String>) -> Vec<(String, String)> {
    table.into_iter().collect()
}

/// One `[[mcp]]` entry, read as written rather than as parsed types:
/// the label's grammar is `kernel`'s answer, and a deserializer that
/// enforced it here would be the second place that rule lives.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct McpSection {
    label: String,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    args: Vec<String>,
    /// What the child process is started with. A table rather than a
    /// list of `NAME=value` lines: the file states one name once, and
    /// nothing here has to split a string a person wrote.
    #[serde(default)]
    env: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    headers: std::collections::BTreeMap<String, String>,
    /// Which of the two ways a url is reached. Absent means `http`.
    #[serde(default)]
    transport: Option<ReachedBy>,
}

/// The two ways a url answers. A closed set rather than free text, so a
/// misspelling is refused where it is written instead of becoming a
/// server nobody can reach.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReachedBy {
    Http,
    Sse,
}
