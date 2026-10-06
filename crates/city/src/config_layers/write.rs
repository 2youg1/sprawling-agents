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
//! every one of them goes through [`change`]: the file is held against
//! every other writer in this process while it is read, and the new
//! content replaces it whole. Write paths that each read and truncate
//! on their own could each drop what another had just saved.
//!
//! Specified by `crates/city/spec/ConfigLayers.lean` §8-4b.

use std::path::Path;

use kernel::config::{SearchConfiguration, SecondThreshold};
use kernel::{
    Address, AxCode, AxError, B3Hash, Effort, KeepWarm, McpServer, McpTransport, SandboxLimits,
};

use super::{ConfigLayer, Layer, path};
use crate::document;

/// Writes the sandbox limits into one scope's own configuration.
///
/// Same door as [`super::write_session`] and the same reason: the ladder that
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

/// Writes the context reminder's second rung into one scope's own
/// configuration.
///
/// Same door as [`super::write_session`] and the same reason: the ladder that
/// resolves city → building → room is the only store. The value arrives
/// already taken by `kernel::config::SecondThreshold`'s one construction
/// point, so an out-of-domain percent is refused where the file is
/// parsed, never here. Other keys are preserved, because a person may
/// have written them.
///
/// # Errors
/// Propagates a file that exists and cannot be read or parsed, and a
/// directory that cannot be written.
pub fn write_second_threshold(
    city_root: &Path,
    addr: &Address,
    layer: Layer,
    threshold: SecondThreshold,
) -> Result<(), AxError> {
    change(city_root, addr, layer, Change::SecondThreshold(threshold))
}

/// Writes the external servers this scope reaches into its own
/// configuration.
///
/// An empty list is a scope that reaches none, written rather than
/// omitted: leaving the key out would inherit the layer above, and a
/// person who removed the last server did not mean to inherit one.
///
/// # Errors
/// Refuses with `E_CONFIG_INVALID`, before any byte is written, every
/// row the reader would refuse for the same reason: an environment value
/// or a header that carries a credential instead of a reference to the
/// vault, and a url that is not a plain http or https address. Propagates a file that exists
/// and cannot be read or parsed, and a directory that cannot be
/// written.
pub fn write_mcp(
    city_root: &Path,
    addr: &Address,
    layer: Layer,
    servers: &[McpServer],
) -> Result<(), AxError> {
    super::mcp::validate(servers)?;
    change(city_root, addr, layer, Change::Mcp(servers))
}

/// What one edit states. Exhaustive, so the section a value is written
/// under is decided in one match rather than once per write face.
pub(super) enum Change<'a> {
    Session {
        model: &'a str,
        effort: Option<Effort>,
    },
    /// The record a session wrote at its own address, removed: the
    /// inverse of `Session`, and the one change here that takes
    /// something out of a file rather than stating it.
    Forget,
    Sandbox(&'a SandboxLimits),
    Mcp(&'a [McpServer]),
    /// The context reminder's second rung. Arrives already taken by
    /// `SecondThreshold`'s one construction point: a raw percent is
    /// refused where it is parsed, never where it is written.
    SecondThreshold(SecondThreshold),
    /// The identity version a session froze (`crates/city/spec/Identity.lean` §8-33).
    Naming(B3Hash),
    /// Whether this layer keeps a warm cache (`crates/city/spec/Policy.lean` §8-34).
    KeepWarm(KeepWarm),
    /// How hard the runs under this layer think, stated for every
    /// session that does not choose. Only the city's own layer is written
    /// this way: a room's `[model]` is a session's record (8-14).
    Effort(Effort),
    /// Which supplier `web_search` reaches (§8-4c), already judged by the
    /// reader's own check.
    Search(&'a SearchConfiguration),
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
            Change::Forget => {
                // Only the two keys `Change::Session` writes. The table
                // is where a session states its shape, so a file with no
                // `[model]` has nothing to forget; a file where `model`
                // is not a table is one this build cannot read, and it
                // is refused here rather than silently skipped.
                let emptied = match document.get_mut("model") {
                    Some(toml::Value::Table(section)) => {
                        section.remove("name");
                        section.remove("effort");
                        section.is_empty()
                    }
                    Some(_) => return Err(refuse_file(file, "`[model]` is not a section")),
                    None => false,
                };
                // An empty `[model]` table is not a statement, and
                // leaving it would keep a reader asking whether a
                // session that states nothing is a session at all.
                if emptied {
                    document.remove("model");
                }
                // The names a session froze go with the shape it froze:
                // the next session takes the names as they are then.
                document.remove("identity");
            }
            Change::Sandbox(limits) => {
                let spelled = toml::Value::try_from(*limits)
                    .map_err(|err| refuse_file(file, &err.to_string()))?;
                document.insert("sandbox".to_owned(), spelled);
            }
            Change::Search(configuration) => {
                document.insert(
                    "search".to_owned(),
                    super::search::spelled(configuration, file)?,
                );
            }
            Change::Mcp(servers) => {
                document.insert("mcp".to_owned(), toml::Value::Array(rows(servers)));
            }
            Change::SecondThreshold(threshold) => {
                let percent = i64::try_from(u64::from(*threshold))
                    .map_err(|err| refuse_file(file, &err.to_string()))?;
                table(document, "context", file)?
                    .insert("second_threshold".to_owned(), toml::Value::Integer(percent));
            }
            Change::KeepWarm(setting) => {
                let spelled = toml::Value::try_from(*setting)
                    .map_err(|err| refuse_file(file, &err.to_string()))?;
                table(document, "cache", file)?.insert("keep_warm".to_owned(), spelled);
            }
            Change::Effort(effort) => {
                table(document, "model", file)?
                    .insert("effort".to_owned(), spelled(*effort, file)?);
            }
            Change::Naming(version) => {
                table(document, "identity", file)?.insert(
                    "version".to_owned(),
                    toml::Value::String(version.to_string()),
                );
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
    change_at(&path(city_root, addr, layer)?, change)
}

/// [`change`], for a layer named by its file: the city's own layer has
/// no address to name it by.
pub(super) fn change_at(file: &Path, change: Change<'_>) -> Result<(), AxError> {
    document::edit(file, |held| {
        let mut document = read_document(file)?;
        change.state(&mut document, file)?;
        let rendered =
            toml::to_string_pretty(&document).map_err(|err| refuse_file(file, &err.to_string()))?;
        // The reader is the file's grammar: bytes it would refuse are not
        // written, so no write leaves a layer every run then refuses.
        ConfigLayer::parse(&rendered).map_err(|err| refuse_file(file, err.subject()))?;
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
