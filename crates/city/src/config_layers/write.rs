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

use kernel::{Address, AxCode, AxError, Effort, McpServer, McpTransport, SandboxLimits};

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
/// Propagates a file that exists and cannot be read or parsed, and a
/// directory that cannot be written.
pub fn write_mcp(
    city_root: &Path,
    addr: &Address,
    layer: Layer,
    servers: &[McpServer],
) -> Result<(), AxError> {
    change(city_root, addr, layer, Change::Mcp(servers))
}

/// What one edit states. Exhaustive, so the section a value is written
/// under is decided in one match rather than once per write face.
enum Change<'a> {
    Effort(Effort),
    Sandbox(&'a SandboxLimits),
    Mcp(&'a [McpServer]),
}

impl Change<'_> {
    /// States this change in `document`, leaving every other key as the
    /// person who wrote it left it.
    fn state(&self, document: &mut toml::Table, file: &Path) -> Result<(), AxError> {
        match self {
            Change::Effort(effort) => {
                let model = document
                    .entry("model".to_owned())
                    .or_insert_with(|| toml::Value::Table(toml::Table::new()));
                let toml::Value::Table(model) = model else {
                    return Err(refuse_file(file, "`[model]` is not a section"));
                };
                let spelled = toml::Value::try_from(*effort)
                    .map_err(|err| refuse_file(file, &err.to_string()))?;
                model.insert("effort".to_owned(), spelled);
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

/// Reads one layer's file, states the change in it, and puts it back
/// whole.
///
/// The one write path into a `CONFIG.toml`. It holds the file while it
/// reads and writes, so a second session editing the same file waits
/// rather than deciding from an original that is already stale, and the
/// replacement is atomic, so a run reading the file meets the version
/// before this change or the version after it.
fn change(
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
