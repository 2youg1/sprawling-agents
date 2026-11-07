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
    Address, AxCode, AxError, B3Hash, Effort, KeepWarm, McpDefinition, McpReference, McpServer,
    McpTransport, SandboxLimits,
};

use super::{ConfigLayer, Layer, path};
use crate::document;

/// Converts equal building-local definitions into references and marks
/// genuinely different local definitions as deliberate conflicts.
///
/// The whole edit is atomic and is read back through the ordinary parser.
pub fn migrate_mcp(city_root: &Path, building: &Address) -> Result<bool, AxError> {
    let file = path(city_root, building, Layer::Building)?;
    if !file.exists() {
        return Ok(false);
    }
    let city = super::ladder::stated(
        &kernel::layout::CityLayout::new(city_root).city_config(),
        Layer::City,
    )?;
    let mut migrated = false;
    document::edit(&file, |held| {
        let mut document = read_document(&file)?;
        let original = toml::to_string_pretty(&document)
            .map_err(|err| refuse_file(&file, &err.to_string()))?;
        let layer =
            ConfigLayer::parse(&original).map_err(|err| refuse_file(&file, err.subject()))?;
        let mut references = layer
            .mcp()
            .and_then(|mcp| mcp.references.clone())
            .unwrap_or_default();
        let mut changed = false;
        let mut definitions = Vec::new();
        for definition in layer
            .mcp()
            .and_then(|mcp| mcp.definitions.clone())
            .unwrap_or_default()
        {
            let farther = city.mcp().and_then(|mcp| {
                mcp.definitions
                    .as_ref()?
                    .iter()
                    .find(|candidate| candidate.server.label == definition.server.label)
            });
            match farther {
                Some(candidate) if candidate.server == definition.server => {
                    references.push(McpReference::legacy(definition.server.label));
                    changed = true;
                }
                Some(_) if !definition.conflict => {
                    definitions.push(McpDefinition {
                        conflict: true,
                        ..definition
                    });
                    changed = true;
                }
                Some(_) | None => definitions.push(definition),
            }
        }
        if !changed {
            return Ok(());
        }
        migrated = true;
        references.sort_by(|left, right| left.label.as_str().cmp(right.label.as_str()));
        references.dedup_by(|left, right| left.label == right.label);
        document.insert(
            super::mcp::MCP_REFERENCE_FIELD.to_owned(),
            toml::Value::Array(reference_rows(&references)),
        );
        if definitions.is_empty() {
            document.remove("mcp");
        } else {
            let servers = definitions
                .iter()
                .map(|definition| definition.server.clone())
                .collect::<Vec<_>>();
            let mut rows = rows(&servers);
            for (row, definition) in rows.iter_mut().zip(definitions.iter()) {
                if definition.conflict
                    && let toml::Value::Table(row) = row
                {
                    row.insert("conflict".to_owned(), toml::Value::Boolean(true));
                }
            }
            document.insert("mcp".to_owned(), toml::Value::Array(rows));
        }
        let rendered = toml::to_string_pretty(&document)
            .map_err(|err| refuse_file(&file, &err.to_string()))?;
        ConfigLayer::parse(&rendered).map_err(|err| refuse_file(&file, err.subject()))?;
        held.replace(rendered.as_bytes())
    })?;
    Ok(migrated)
}

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

/// Replaces a building's server references and their pins.
pub fn write_mcp_references(
    city_root: &Path,
    building: &Address,
    references: &[McpReference],
) -> Result<(), AxError> {
    super::check_mcp_references(city_root, building, references)?;
    change(
        city_root,
        building,
        Layer::Building,
        Change::McpReferences(references),
    )
}

/// Replaces a building's Reading Room pins.
pub fn write_skill_pins(
    city_root: &Path,
    building: &Address,
    pins: &std::collections::BTreeMap<String, B3Hash>,
) -> Result<(), AxError> {
    change(
        city_root,
        building,
        Layer::Building,
        Change::SkillPins(pins),
    )
}

/// Fills only the missing pins of a legacy reference discovered at run start.
pub fn migrate_mcp_reference(
    city_root: &Path,
    building: &Address,
    reference: &McpReference,
) -> Result<(), AxError> {
    change(
        city_root,
        building,
        Layer::Building,
        Change::MigrateMcpReference(reference),
    )
}

/// Fills one missing Reading Room pin without moving an accepted version.
pub fn migrate_skill_pin(
    city_root: &Path,
    building: &Address,
    name: &str,
    digest: B3Hash,
) -> Result<(), AxError> {
    change(
        city_root,
        building,
        Layer::Building,
        Change::MigrateSkillPin { name, digest },
    )
}

/// What one edit states. Exhaustive, so the section a value is written
/// under is decided in one match rather than once per write face.
pub(super) enum Change<'a> {
    Session {
        model: &'a str,
        effort: Option<Effort>,
        generation: Option<&'a kernel::FrozenGeneration>,
    },
    /// The record a session wrote at its own address, removed: the
    /// inverse of `Session`, and the one change here that takes
    /// something out of a file rather than stating it.
    Forget,
    Sandbox(&'a SandboxLimits),
    Mcp(&'a [McpServer]),
    McpReferences(&'a [McpReference]),
    SkillPins(&'a std::collections::BTreeMap<String, B3Hash>),
    MigrateMcpReference(&'a McpReference),
    MigrateSkillPin {
        name: &'a str,
        digest: B3Hash,
    },
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
    /// One `[[agent]]` row the consent card wrote (§8-4f), replacing the
    /// row with its id.
    Agent(&'a super::AgentRow),
    /// A room's resident named as this agent: the session record taken
    /// out as [`Change::Forget`] takes it, then `[resident] harness`.
    Seat(&'a str),
    /// A layer's whole `[catalog]` table (§8-4g), already judged by the
    /// reader's own check; an empty one takes the key out.
    Catalog(&'a kernel::config::ExposureTable),
}

impl Change<'_> {
    /// States this change in `document`, leaving every other key as the
    /// person who wrote it left it.
    fn state(&self, document: &mut toml::Table, file: &Path) -> Result<(), AxError> {
        match self {
            Change::Session {
                model,
                effort,
                generation,
            } => {
                let section = table(document, "model", file)?;
                section.insert("name".to_owned(), toml::Value::String((*model).to_owned()));
                if let Some(effort) = effort {
                    section.insert("effort".to_owned(), spelled(*effort, file)?);
                }
                state_generation(section, *generation, file)?;
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
                        for key in GENERATION_KEYS {
                            section.remove(key);
                        }
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
            Change::Catalog(table) => match super::catalog::spelled(table, file)? {
                Some(spelled) => {
                    document.insert("catalog".to_owned(), spelled);
                }
                None => {
                    document.remove("catalog");
                }
            },
            Change::Mcp(servers) => {
                document.insert("mcp".to_owned(), toml::Value::Array(rows(servers)));
            }
            Change::McpReferences(references) => {
                document.insert(
                    super::mcp::MCP_REFERENCE_FIELD.to_owned(),
                    toml::Value::Array(reference_rows(references)),
                );
            }
            Change::SkillPins(pins) => {
                let spelled = pins
                    .iter()
                    .map(|(name, digest)| (name.clone(), toml::Value::String(digest.to_string())))
                    .collect();
                table(document, "skills", file)?
                    .insert("pins".to_owned(), toml::Value::Table(spelled));
            }
            Change::MigrateMcpReference(reference) => {
                migrate_reference(document, reference, file)?;
            }
            Change::MigrateSkillPin { name, digest } => {
                let pins = table(document, "skills", file)?;
                let toml::Value::Table(pins) = pins
                    .entry("pins".to_owned())
                    .or_insert_with(|| toml::Value::Table(toml::Table::new()))
                else {
                    return Err(refuse_file(file, "`[skills.pins]` is not a table"));
                };
                pins.entry((*name).to_owned())
                    .or_insert_with(|| toml::Value::String(digest.to_string()));
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
            Change::Agent(row) => {
                let rows = document
                    .entry("agent".to_owned())
                    .or_insert_with(|| toml::Value::Array(Vec::new()));
                let toml::Value::Array(rows) = rows else {
                    return Err(refuse_file(
                        file,
                        "`agent` is not a list of `[[agent]]` rows",
                    ));
                };
                let spelled = toml::Value::Table(super::agents::spelled(row));
                let same = |held: &toml::Value| {
                    held.get("id").and_then(toml::Value::as_str) == Some(row.id.as_str())
                };
                match rows.iter_mut().find(|held| same(held)) {
                    Some(held) => *held = spelled,
                    None => rows.push(spelled),
                }
            }
            Change::Seat(id) => {
                Change::Forget.state(document, file)?;
                table(document, "resident", file)?
                    .insert("harness".to_owned(), toml::Value::String((*id).to_owned()));
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

const GENERATION_KEYS: [&str; 5] = ["temperature", "top_p", "top_k", "max_tokens", "overrides"];

/// Replace only the generation keys a session owns, preserving every
/// other setting in `[model]` and the rest of the file.
fn state_generation(
    section: &mut toml::Table,
    generation: Option<&kernel::FrozenGeneration>,
    file: &Path,
) -> Result<(), AxError> {
    for key in GENERATION_KEYS {
        section.remove(key);
    }
    let Some(frozen) = generation else {
        return Ok(());
    };
    for (key, value) in [
        ("temperature", &frozen.settings.temperature),
        ("top_p", &frozen.settings.top_p),
        ("top_k", &frozen.settings.top_k),
    ] {
        if let Some(text) = value {
            section.insert(key.to_owned(), toml::Value::String(text.clone()));
        }
    }
    if let Some(ceiling) = frozen.max_tokens {
        let count =
            i64::try_from(ceiling.get()).map_err(|err| refuse_file(file, &err.to_string()))?;
        section.insert("max_tokens".to_owned(), toml::Value::Integer(count));
    }
    if !frozen.overrides.is_empty() {
        section.insert(
            "overrides".to_owned(),
            toml::Value::Array(
                frozen
                    .overrides
                    .iter()
                    .map(|(pointer, value)| {
                        toml::Value::Array(vec![
                            toml::Value::String(pointer.clone()),
                            toml::Value::String(value.clone()),
                        ])
                    })
                    .collect(),
            ),
        );
    }
    Ok(())
}

/// The `[model]` table, made if this file has none: one place the
/// section is found or created, so the values written into it cannot
/// disagree about which table holds them.
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

/// Reference rows in the file's grammar, with absent legacy pins omitted.
fn reference_rows(references: &[McpReference]) -> Vec<toml::Value> {
    references
        .iter()
        .map(|reference| {
            let mut row = toml::Table::new();
            row.insert(
                "label".to_owned(),
                toml::Value::String(reference.label.as_str().to_owned()),
            );
            if let Some(definition) = reference.definition {
                row.insert(
                    "definition".to_owned(),
                    toml::Value::String(definition.to_string()),
                );
            }
            if let Some(tools) = &reference.tools {
                row.insert(
                    "tools".to_owned(),
                    toml::Value::Table(
                        tools
                            .iter()
                            .map(|(name, digest)| {
                                (name.clone(), toml::Value::String(digest.to_string()))
                            })
                            .collect(),
                    ),
                );
            }
            toml::Value::Table(row)
        })
        .collect()
}

/// Migrates one reference without overwriting pins a person saved meanwhile.
fn migrate_reference(
    document: &mut toml::Table,
    reference: &McpReference,
    file: &Path,
) -> Result<(), AxError> {
    let rows = document
        .entry(super::mcp::MCP_REFERENCE_FIELD.to_owned())
        .or_insert_with(|| toml::Value::Array(Vec::new()));
    let toml::Value::Array(rows) = rows else {
        return Err(refuse_file(
            file,
            &format!(
                "`{0}` is not a list of `[[{0}]]` rows",
                super::mcp::MCP_REFERENCE_FIELD
            ),
        ));
    };
    let label = reference.label.as_str();
    if let Some(row) = rows
        .iter_mut()
        .find(|row| row.get("label").and_then(toml::Value::as_str) == Some(label))
    {
        let toml::Value::Table(row) = row else {
            return Err(refuse_file(
                file,
                &format!(
                    "a `[[{}]]` row is not a table",
                    super::mcp::MCP_REFERENCE_FIELD
                ),
            ));
        };
        if !row.contains_key("definition")
            && let Some(definition) = reference.definition
        {
            row.insert(
                "definition".to_owned(),
                toml::Value::String(definition.to_string()),
            );
        }
        if !row.contains_key("tools")
            && let Some(tools) = &reference.tools
        {
            row.insert(
                "tools".to_owned(),
                toml::Value::Table(
                    tools
                        .iter()
                        .map(|(name, digest)| {
                            (name.clone(), toml::Value::String(digest.to_string()))
                        })
                        .collect(),
                ),
            );
        }
    } else {
        rows.extend(reference_rows(std::slice::from_ref(reference)));
    }
    Ok(())
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
