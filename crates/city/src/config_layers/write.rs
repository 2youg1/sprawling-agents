// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Writing a value back into the layer that governs it.
//!
//! The reader's ladder is the only store: a choice made in a session is
//! written into that scope's own `CONFIG.toml`, so what a later run
//! reads and what a person edits by hand are one file.

use std::path::Path;

use kernel::{Address, AxCode, AxError, Effort, McpServer, McpTransport, SandboxLimits};

use super::{Layer, path};

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
    let file = path(city_root, addr, Layer::Resident)?;
    let mut document: toml::Table = match std::fs::read_to_string(&file) {
        Ok(text) => toml::from_str(&text).map_err(|err| refuse_file(&file, &err.to_string()))?,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => toml::Table::new(),
        Err(err) => return Err(refuse_file(&file, &err.to_string())),
    };
    let model = document
        .entry("model".to_owned())
        .or_insert_with(|| toml::Value::Table(toml::Table::new()));
    let toml::Value::Table(model) = model else {
        return Err(refuse_file(&file, "`[model]` is not a section"));
    };
    let spelled =
        toml::Value::try_from(effort).map_err(|err| refuse_file(&file, &err.to_string()))?;
    model.insert("effort".to_owned(), spelled);
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|err| refuse_file(dir, &err.to_string()))?;
    }
    let rendered =
        toml::to_string_pretty(&document).map_err(|err| refuse_file(&file, &err.to_string()))?;
    std::fs::write(&file, rendered).map_err(|err| refuse_file(&file, &err.to_string()))
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
    let file = path(city_root, addr, layer)?;
    let mut document = read_document(&file)?;
    let spelled =
        toml::Value::try_from(limits).map_err(|err| refuse_file(&file, &err.to_string()))?;
    document.insert("sandbox".to_owned(), spelled);
    write_document(&file, &document)
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
    let file = path(city_root, addr, layer)?;
    let mut document = read_document(&file)?;
    // Written as the reader below spells it, not as `McpServer`
    // serialises: the file's grammar is `ConfigFile`'s, and a writer
    // that emitted serde's nesting would produce a file this build
    // refuses to read back.
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
    document.insert("mcp".to_owned(), toml::Value::Array(rows));
    write_document(&file, &document)
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

fn write_document(file: &Path, document: &toml::Table) -> Result<(), AxError> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|err| refuse_file(dir, &err.to_string()))?;
    }
    let rendered =
        toml::to_string_pretty(document).map_err(|err| refuse_file(file, &err.to_string()))?;
    std::fs::write(file, rendered).map_err(|err| refuse_file(file, &err.to_string()))
}

fn refuse_file(path: &Path, why: &str) -> AxError {
    AxError::failure(
        AxCode::ConfigInvalid,
        "write a configuration layer",
        format!("{}: {why}", path.display()),
    )
    .with_recovery("fix that file by hand, or delete it and choose again")
}
