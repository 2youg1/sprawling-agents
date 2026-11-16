// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{
    AxCode, AxError, B3Hash, McpDefinition, McpReference, McpServer, McpTransport,
    ReservedNamespace, SecretRef, ServerLabel,
};
use serde::Deserialize;

use super::refuse::refuse;

const ACTION: &str = "check the servers a layer reaches";

pub(crate) const MCP_REFERENCE_FIELD: &str = "mcp_ref";

pub(crate) fn validate(servers: &[McpServer]) -> Result<(), AxError> {
    servers.iter().try_for_each(|server| {
        unreserved(&server.label)?;
        match &server.transport {
            McpTransport::Stdio { env, .. } => vaulted(server.label.as_str(), Carried::Env, env),
            McpTransport::Http { url, headers } | McpTransport::Sse { url, headers } => {
                check_url(&server.label, url)?;
                vaulted(server.label.as_str(), Carried::Header, headers)
            }
        }
    })
}

fn unreserved(label: &ServerLabel) -> Result<(), AxError> {
    if ReservedNamespace::named(label.as_str()).is_none() {
        return Ok(());
    }
    Err(AxError::failure(
        AxCode::ConfigInvalid,
        ACTION,
        format!(
            "{}: a namespace the city keeps for its own tools",
            label.as_str()
        ),
    )
    .with_recovery(format!(
        "give this server another label; {} are kept for the city's own tools",
        ReservedNamespace::ALL
            .map(ReservedNamespace::as_str)
            .join(", ")
    )))
}

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

const IN_A_HEADER: &str = "keep the secret in the vault and send it in a header whose value is \
     its `secret:realm/name` reference; this file is committed with the project";

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

pub(super) fn servers(entries: Vec<McpSection>) -> Result<Vec<McpDefinition>, AxError> {
    let mut servers: Vec<McpDefinition> = Vec::new();
    for entry in entries {
        let label = ServerLabel::parse(&entry.label)
            .map_err(|err| refuse(format!("{}: {}", entry.label, err.recovery())))?;

        let transport = match (entry.command.as_deref(), entry.url.as_deref()) {
            (Some(command), None) if !command.trim().is_empty() => {

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

        if servers.iter().any(|held| held.server.label == label) {
            return Err(refuse(format!(
                "{}: two servers are named the same in one layer",
                label.as_str()
            )));
        }
        servers.push(McpDefinition {
            server: McpServer { label, transport },
            conflict: entry.conflict,
        });
    }
    validate(
        &servers
            .iter()
            .map(|definition| definition.server.clone())
            .collect::<Vec<_>>(),
    )?;
    Ok(servers)
}

pub(super) fn references(entries: Vec<McpReferenceSection>) -> Result<Vec<McpReference>, AxError> {
    let mut references: Vec<McpReference> = Vec::new();
    for entry in entries {
        let label = ServerLabel::parse(&entry.label)
            .map_err(|err| refuse(format!("{}: {}", entry.label, err.recovery())))?;
        unreserved(&label)?;
        if references.iter().any(|held| held.label == label) {
            return Err(refuse(format!(
                "{}: two references are named the same in one layer",
                label.as_str()
            )));
        }
        references.push(McpReference {
            label,
            definition: entry.definition,
            tools: entry.tools,
        });
    }
    Ok(references)
}

fn listed(table: std::collections::BTreeMap<String, String>) -> Vec<(String, String)> {
    table.into_iter().collect()
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct McpSection {
    label: String,

    #[serde(default)]
    conflict: bool,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    args: Vec<String>,

    #[serde(default)]
    env: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    headers: std::collections::BTreeMap<String, String>,

    #[serde(default)]
    transport: Option<ReachedBy>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReachedBy {
    Http,
    Sse,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct McpReferenceSection {
    label: String,
    #[serde(default)]
    definition: Option<B3Hash>,
    #[serde(default)]
    tools: Option<std::collections::BTreeMap<String, B3Hash>>,
}
