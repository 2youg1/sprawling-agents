// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Writing a value back into the layer that governs it.
//!
//! The reader's ladder is the only store: a choice made in a session is
//! written into that scope's own `CONFIG.toml`, so what a later run
//! reads and what a person edits by hand are one file.
//!
//! Every write is a read-modify-write of a file a person also edits, so
//! all three of them go through [`change`]: the file is held against
//! every other writer in this process while it is read, and the new
//! content replaces it whole. Three write paths, each reading and
//! truncating on its own, could each drop what the other two had just
//! saved.

use std::path::Path;

use kernel::{
    Address, AxCode, AxError, Effort, McpServer, McpTransport, SandboxLimits, SecretRef,
    ServerLabel,
};

use super::{Layer, path};
use crate::document;

/// Writes the thinking effort into one scope's own configuration.
///
/// The layer is where the choice belongs rather than a second store: a
/// session picks an effort once, it is written into that room's
/// `CONFIG.toml`, and the ladder that already resolves city → building →
/// room is what every later run in that room reads. Other keys in the
/// file are preserved, because a person may have written them.
///
/// # Errors
/// Propagates a file that exists and cannot be read or parsed - a
/// configuration this build cannot understand is not one to overwrite -
/// and a directory that cannot be written.
pub fn write_effort(city_root: &Path, addr: &Address, effort: Effort) -> Result<(), AxError> {
    change(city_root, addr, Layer::Resident, Change::Effort(effort))
}

/// Writes the sandbox limits into one scope's own configuration.
///
/// Same door as [`write_effort`] and the same reason: the ladder that
/// resolves city → building → room is already the authority on what a
/// run may reach, and a second store would be a second answer. Other
/// keys are preserved, because a person may have written them.
///
/// # Errors
/// Propagates a file that exists and cannot be read or parsed, and a
/// directory that cannot be written.
pub fn write_sandbox(
    city_root: &Path,
    addr: &Address,
    layer: Layer,
    limits: &SandboxLimits,
) -> Result<(), AxError> {
    change(city_root, addr, layer, Change::Sandbox(limits))
}

/// Writes the external servers this scope reaches into its own
/// configuration.
///
/// An empty list is a scope that reaches none, written rather than
/// omitted: leaving the key out would inherit the layer above, and a
/// person who removed the last server did not mean to inherit one.
///
/// # Errors
/// Refuses with `E_CONFIG_INVALID` when one environment value or one
/// header of one server carries a credential instead of a reference to
/// the vault, before any byte is written. Propagates a file that exists
/// and cannot be read or parsed, and a directory that cannot be
/// written.
pub fn write_mcp(
    city_root: &Path,
    addr: &Address,
    layer: Layer,
    servers: &[McpServer],
) -> Result<(), AxError> {
    for server in servers {
        match &server.transport {
            McpTransport::Stdio { env, .. } => vaulted(&server.label, Carried::Env, env)?,
            McpTransport::Http { headers, .. } | McpTransport::Sse { headers, .. } => {
                vaulted(&server.label, Carried::Header, headers)?;
            }
        }
    }
    change(city_root, addr, layer, Change::Mcp(servers))
}

/// Which table a value sits in, so a refusal names the line a person
/// has to go and edit.
#[derive(Clone, Copy)]
enum Carried {
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
/// problem rather than which server.
///
/// A `secret:realm/name` reference is what this file is for: the value
/// on disk says where the secret is kept, and the assembly layer
/// redeems it when a server is started. Anything else that reads as a
/// credential — by the name in front of it or by the shape of the value
/// itself — is refused here, at the one place that writes this file,
/// because a building's `CONFIG.toml` is committed with the project and
/// a key written into it is a key in every clone of that repository.
/// The same judgment `EnvVarName::parse` already applies to a declared
/// environment name, applied to the value beside it.
fn vaulted(
    label: &ServerLabel,
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
            "write the servers this scope reaches",
            format!(
                "{}: {} `{name}`: {violation}",
                label.as_str(),
                carried.noun()
            ),
        )
        .with_recovery(
            "keep the secret in the vault and write its `secret:realm/name` reference here; \
             this file is committed with the project",
        ));
    }
    Ok(())
}

/// What one edit states. Exhaustive, so the section a value is written
/// under is decided in one match rather than once per write face.
pub(super) enum Change<'a> {
    Session {
        model: &'a str,
        effort: Option<Effort>,
    },
    Effort(Effort),
    Sandbox(&'a SandboxLimits),
    Mcp(&'a [McpServer]),
}

impl Change<'_> {
    /// States this change in `document`, leaving every other key as the
    /// person who wrote it left it.
    fn state(&self, document: &mut toml::Table, file: &Path) -> Result<(), AxError> {
        match self {
            Change::Session { model, effort } => {
                let section = table(document, "model", file)?;
                section.insert("name".to_owned(), toml::Value::String((*model).to_owned()));
                if let Some(effort) = effort {
                    section.insert("effort".to_owned(), spelled(*effort, file)?);
                }
            }
            Change::Effort(effort) => {
                table(document, "model", file)?
                    .insert("effort".to_owned(), spelled(*effort, file)?);
            }
            Change::Sandbox(limits) => {
                let spelled = toml::Value::try_from(*limits)
                    .map_err(|err| refuse_file(file, &err.to_string()))?;
                document.insert("sandbox".to_owned(), spelled);
            }
            Change::Mcp(servers) => {
                document.insert("mcp".to_owned(), toml::Value::Array(rows(servers)));
            }
        }
        Ok(())
    }
}

/// The `[model]` table, made if this file has none: one place the
/// section is found or created, so the three values written into it
/// cannot disagree about which table holds them.
fn table<'d>(
    document: &'d mut toml::Table,
    name: &str,
    file: &Path,
) -> Result<&'d mut toml::Table, AxError> {
    let section = document
        .entry(name.to_owned())
        .or_insert_with(|| toml::Value::Table(toml::Table::new()));
    let toml::Value::Table(held) = section else {
        return Err(refuse_file(file, &format!("`[{name}]` is not a section")));
    };
    Ok(held)
}

/// One effort as a configuration file spells it.
fn spelled(effort: Effort, file: &Path) -> Result<toml::Value, AxError> {
    toml::Value::try_from(effort).map_err(|err| refuse_file(file, &err.to_string()))
}

/// Reads one layer's file, states the change in it, and puts it back
/// whole.
///
/// The one write path into a `CONFIG.toml`. It holds the file while it
/// reads and writes, so a second session editing the same file waits
/// rather than deciding from an original that is already stale, and the
/// replacement is atomic, so a run reading the file meets the version
/// before this change or the version after it.
pub(super) fn change(
    city_root: &Path,
    addr: &Address,
    layer: Layer,
    change: Change<'_>,
) -> Result<(), AxError> {
    let file = path(city_root, addr, layer)?;
    document::edit(&file, |held| {
        let mut document = read_document(&file)?;
        change.state(&mut document, &file)?;
        let rendered = toml::to_string_pretty(&document)
            .map_err(|err| refuse_file(&file, &err.to_string()))?;
        held.replace(rendered.as_bytes())
    })
}

/// The `[[mcp]]` rows as the reader below spells them, not as
/// `McpServer` serialises: the file's grammar is `ConfigFile`'s, and a
/// writer that emitted serde's nesting would produce a file this build
/// refuses to read back.
fn rows(servers: &[McpServer]) -> Vec<toml::Value> {
    let mut rows = Vec::with_capacity(servers.len());
    for server in servers {
        let mut row = toml::Table::new();
        row.insert(
            "label".to_owned(),
            toml::Value::String(server.label.as_str().to_owned()),
        );
        match &server.transport {
            McpTransport::Stdio { command, args, env } => {
                row.insert("command".to_owned(), toml::Value::String(command.clone()));
                row.insert(
                    "args".to_owned(),
                    toml::Value::Array(args.iter().cloned().map(toml::Value::String).collect()),
                );
                row.insert("env".to_owned(), toml::Value::Table(tabled(env)));
            }
            McpTransport::Http { url, headers } => {
                row.insert("url".to_owned(), toml::Value::String(url.clone()));
                row.insert("headers".to_owned(), toml::Value::Table(tabled(headers)));
            }
            McpTransport::Sse { url, headers } => {
                row.insert("url".to_owned(), toml::Value::String(url.clone()));
                // Stated rather than left to the default, because the
                // default is `http` and a stream written without this
                // key would be read back as the other transport.
                row.insert(
                    "transport".to_owned(),
                    toml::Value::String("sse".to_owned()),
                );
                row.insert("headers".to_owned(), toml::Value::Table(tabled(headers)));
            }
        }
        rows.push(toml::Value::Table(row));
    }
    rows
}

/// The reader's shape for a configured table: one name once, which is
/// what makes the file readable back through `ConfigLayer::parse`.
fn tabled(pairs: &[(String, String)]) -> toml::Table {
    let mut table = toml::Table::new();
    for (name, value) in pairs {
        table.insert(name.clone(), toml::Value::String(value.clone()));
    }
    table
}

fn read_document(file: &Path) -> Result<toml::Table, AxError> {
    match std::fs::read_to_string(file) {
        Ok(text) => toml::from_str(&text).map_err(|err| refuse_file(file, &err.to_string())),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(toml::Table::new()),
        Err(err) => Err(refuse_file(file, &err.to_string())),
    }
}

fn refuse_file(path: &Path, why: &str) -> AxError {
    AxError::failure(
        AxCode::ConfigInvalid,
        "write a configuration layer",
        format!("{}: {why}", path.display()),
    )
    .with_recovery("fix that file by hand, or delete it and choose again")
}

#[cfg(test)]
mod tests;
