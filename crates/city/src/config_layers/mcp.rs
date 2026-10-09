// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The `[[mcp]]` table: the servers one layer reaches, each read into
//! exactly one transport.
//!
//! Specified by `crates/city/spec/ConfigLayers.lean` §8-4 and §8-4b.

use kernel::{AxCode, AxError, McpServer, McpTransport, SecretRef, ServerLabel};
use serde::Deserialize;

use super::refuse::refuse;

/// What a refusal of this judgement says it was doing: the same words
/// whether the row came from a person's file or from the settings page.
const ACTION: &str = "check the servers a layer reaches";

/// Judges every server one layer reaches, for the reader and the writer
/// alike: a value in `env` or `headers` that is a credential rather
/// than a vault reference, and a url that is not a plain http or https
/// address, are refused.
///
/// One judgement for both faces, because a building's `CONFIG.toml` is
/// committed with the project: a key the reader accepted from a file a
/// person wrote by hand is a key in every clone of that repository, just
/// as one the settings page had written would be.
///
/// # Errors
/// `E_CONFIG_INVALID`, naming the server and the value, never the value's
/// bytes.
pub(crate) fn validate(servers: &[McpServer]) -> Result<(), AxError> {
    servers
        .iter()
        .try_for_each(|server| match &server.transport {
            McpTransport::Stdio { env, .. } => vaulted(server.label.as_str(), Carried::Env, env),
            McpTransport::Http { url, headers } | McpTransport::Sse { url, headers } => {
                check_url(&server.label, url)?;
                vaulted(server.label.as_str(), Carried::Header, headers)
            }
        })
}

/// Judges one configured address. `url::Url` reads it by the WHATWG
/// rules, so a credential hidden in userinfo or behind a percent-encoded
/// parameter name is seen (city D26); a query or fragment that carries
/// no credential is kept byte for byte, because the file keeps the text
/// a person wrote and only the judgement reads the parsed form.
///
/// # Errors
/// `E_CONFIG_INVALID` for text that is not an absolute url, a scheme other
/// than http or https, a missing host, userinfo, a query parameter whose
/// decoded name reads as a credential or whose decoded value has a
/// credential's shape, and credential-shaped bytes anywhere in the text.
pub(crate) fn check_url(label: &ServerLabel, url: &str) -> Result<(), AxError> {
    let unreachable = |why: String| {
        AxError::failure(
            AxCode::ConfigInvalid,
            ACTION,
            format!("{}: the url {why}", label.as_str()),
        )
        .with_recovery("write an absolute `http` or `https` address with a host")
    };
    let parsed =
        url::Url::parse(url).map_err(|err| unreachable(format!("does not parse ({err})")))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(unreachable(format!("uses `{}`", parsed.scheme())));
    }
    if parsed.host_str().is_none_or(str::is_empty) {
        return Err(unreachable("names no host".to_owned()));
    }
    let named = |violation: String| {
        AxError::failure(
            AxCode::ConfigInvalid,
            ACTION,
            format!("{}: the url {violation}", label.as_str()),
        )
        .with_recovery(IN_A_HEADER)
    };
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(named("carries userinfo".to_owned()));
    }
    if let Some((name, _)) = parsed.query_pairs().find(|(name, value)| {
        kernel::secret::names_a_credential(name)
            || !kernel::secret::scan(value.as_bytes()).is_empty()
    }) {
        return Err(named(format!(
            "query parameter `{name}` carries a credential"
        )));
    }
    if !kernel::secret::scan(url.as_bytes()).is_empty() {
        return Err(named("has the shape of a credential".to_owned()));
    }
    Ok(())
}

/// Where a credential a url carried belongs instead.
const IN_A_HEADER: &str = "keep the secret in the vault and send it in a header whose value is \
     its `secret:realm/name` reference; this file is committed with the project";

/// Which table a value sits in, so a refusal names the line a person
/// has to go and edit.
#[derive(Clone, Copy)]
pub(super) enum Carried {
    Env,
    Header,
}

impl Carried {
    fn noun(self) -> &'static str {
        match self {
            Carried::Env => "environment value",
            Carried::Header => "header",
        }
    }
}

/// Refuses one pair at a time, so the refusal names which value is the
/// problem rather than which server or agent `owner` names.
///
/// A `secret:realm/name` reference is what this file is for: the value
/// on disk says where the secret is kept, and the assembly layer
/// redeems it when a server is started. Anything else that reads as a
/// credential — by the name in front of it or by the shape of the value
/// itself — is refused. What reads as a credential is
/// `kernel::secret`'s answer, the same one `EnvVarName::parse` gives
/// for a declared environment name.
pub(super) fn vaulted(
    owner: &str,
    carried: Carried,
    pairs: &[(String, String)],
) -> Result<(), AxError> {
    for (name, value) in pairs {
        if SecretRef::parse(value).is_ok() {
            continue;
        }
        let named = kernel::secret::names_a_credential(name);
        let shaped = !kernel::secret::scan(value.as_bytes()).is_empty();
        let violation = match (named, shaped) {
            (true, _) => "the name reads as a credential",
            (false, true) => "the value has the shape of a credential",
            (false, false) => continue,
        };
        return Err(AxError::failure(
            AxCode::ConfigInvalid,
            ACTION,
            format!("{owner}: {} `{name}`: {violation}", carried.noun()),
        )
        .with_recovery(
            "keep the secret in the vault and write its `secret:realm/name` reference here; \
             this file is committed with the project",
        ));
    }
    Ok(())
}

/// Reads one layer's `[[mcp]]` entries.
///
/// # Errors
/// Refuses a label `kernel` does not accept, a row that names both a
/// command and a url or neither, a stream chosen for a command, two
/// rows under one label, and every row [`validate`] refuses.
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
    validate(&servers)?;
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
