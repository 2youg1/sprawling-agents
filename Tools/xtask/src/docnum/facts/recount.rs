// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use crate::budget::REGISTER;
use crate::members::members;
use crate::report::XtaskError;
use crate::walk;

use super::{register_row, root_manifest};

pub(super) fn wire_tags(kind: &str) -> Result<String, XtaskError> {
    let schema = wire::wire_schema();
    let members = schema
        .get("$defs")
        .and_then(|defs| defs.get(kind))
        .and_then(|family| family.get("oneOf"))
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| XtaskError::Doc {
            file: "wire::wire_schema".to_owned(),
            msg: format!("`$defs.{kind}` is not a `oneOf` of variants"),
        })?;
    let mut tags = Vec::with_capacity(members.len());
    for member in members {
        if let Some(properties) = member
            .get("properties")
            .and_then(serde_json::Value::as_object)
        {
            tags.extend(properties.keys().map(|tag| format!("`{tag}`")));
            continue;
        }
        if let Some(only) = member.get("const").and_then(serde_json::Value::as_str) {
            tags.push(format!("`{only}`"));
            continue;
        }
        let listed = member
            .get("enum")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| XtaskError::Doc {
                file: "wire::wire_schema".to_owned(),
                msg: format!("a `{kind}` variant is neither an object nor a listed tag"),
            })?;
        for tag in listed {
            let Some(tag) = tag.as_str() else {
                return Err(XtaskError::Doc {
                    file: "wire::wire_schema".to_owned(),
                    msg: format!("a `{kind}` tag is not a string"),
                });
            };
            tags.push(format!("`{tag}`"));
        }
    }
    if tags.is_empty() {
        return Err(XtaskError::Doc {
            file: "wire::wire_schema".to_owned(),
            msg: format!("`$defs.{kind}` declares no variant"),
        });
    }
    Ok(tags.join(", "))
}

pub(super) fn dependency_count(root: &Path) -> Result<String, XtaskError> {
    Ok(crate::budget::lockfile_packages(root)?.to_string())
}

pub(super) fn files_under(root: &Path, inside: &str, what: &str) -> Result<String, XtaskError> {
    let mut found = 0_usize;
    for member in members(root)? {
        let base = root.join(&member.dir).join(inside);
        if !base.is_dir() {
            continue;
        }
        found = found.saturating_add(walk::files_with_ext(&base, &["rs"])?.len());
    }
    refuse_if_empty(found, inside, what)
}

pub(super) fn in_directory(root: &Path, dir: &str, what: &str) -> Result<String, XtaskError> {
    let base = root.join(dir);
    if !base.is_dir() {
        return Err(XtaskError::Doc {
            file: dir.to_owned(),
            msg: format!("no such directory, so no {what} can be counted"),
        });
    }
    refuse_if_empty(walk::files_with_ext(&base, &["rs"])?.len(), dir, what)
}

pub(super) fn citysim_scenarios(root: &Path) -> Result<String, XtaskError> {
    let harness = "Tools/citysim/tests/scenarios/main.rs";
    let text = walk::read_text(&root.join(harness))?;
    let found = text
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            line.strip_prefix("mod ")?.strip_suffix(';')
        })
        .filter(|name| {
            !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
        .count();
    refuse_if_empty(found, harness, "scenario file")
}

pub(super) fn dep_version(root: &Path, crate_name: &str) -> Result<String, XtaskError> {
    let mut found: Vec<(String, String)> = Vec::new();
    if let Some(pinned) = pinned_in(&root_manifest(root)?, crate_name) {
        found.push(("Cargo.toml".to_owned(), pinned));
    }
    for member in members(root)? {
        let manifest = root.join(&member.dir).join("Cargo.toml");
        if !manifest.is_file() {
            continue;
        }
        let rel = walk::rel(root, &manifest);
        let text = walk::read_text(&manifest)?;
        let parsed: toml::Value = toml::from_str(&text).map_err(|err| XtaskError::Doc {
            file: rel.clone(),
            msg: format!("this manifest does not parse: {err}"),
        })?;
        if let Some(pinned) = pinned_in(&parsed, crate_name) {
            found.push((rel, pinned));
        }
    }
    let Some((_, first)) = found.first() else {
        return Err(XtaskError::Doc {
            file: "Cargo.toml".to_owned(),
            msg: format!("no manifest in this workspace pins `{crate_name}`"),
        });
    };
    if let Some((where_else, other)) = found.iter().find(|(_, version)| version != first) {
        return Err(XtaskError::Doc {
            file: where_else.clone(),
            msg: format!("`{crate_name}` is pinned at {other} here and at {first} elsewhere"),
        });
    }
    Ok(first.clone())
}

fn pinned_in(manifest: &toml::Value, crate_name: &str) -> Option<String> {
    let direct = manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(|table| version_of(table, crate_name))
        .or_else(|| {
            manifest
                .get("dependencies")
                .and_then(|table| version_of(table, crate_name))
        });
    if direct.is_some() {
        return direct;
    }
    let targets = manifest.get("target")?.as_table()?;
    targets.values().find_map(|arm| {
        arm.get("dependencies")
            .and_then(|table| version_of(table, crate_name))
    })
}

fn version_of(table: &toml::Value, crate_name: &str) -> Option<String> {
    let entry = table.get(crate_name)?;
    match entry {
        toml::Value::String(version) => Some(version.clone()),
        other => other.get("version")?.as_str().map(str::to_owned),
    }
}

pub(super) fn grouped_bytes(root: &Path, row: &str, field: &str) -> Result<String, XtaskError> {
    Ok(format!("{} B", grouped(bytes_of(root, row, field)?)))
}

pub(super) fn figure(root: &Path, argument: &str) -> Result<String, XtaskError> {
    let (row, field) = argument.split_once('.').ok_or_else(|| XtaskError::Doc {
        file: REGISTER.to_owned(),
        msg: format!("`{argument}` names no field: write it as `row.field`"),
    })?;
    Ok(grouped(bytes_of(root, row, field)?))
}

pub(super) fn headroom(root: &Path, row: &str) -> Result<String, XtaskError> {
    let budget = bytes_of(root, row, "budget_bytes")?;
    let reading = bytes_of(root, row, "best_bytes")?;

    let tenths = budget
        .checked_mul(10)
        .and_then(|scaled| scaled.checked_add(reading.checked_div(2)?))
        .and_then(|rounded| rounded.checked_div(reading))
        .ok_or_else(|| XtaskError::Doc {
            file: REGISTER.to_owned(),
            msg: format!("[{row}] has no headroom to state: its reading is zero"),
        })?;
    Ok(format!(
        "{}.{}\u{d7}",
        tenths.saturating_div(10),
        tenths % 10
    ))
}

fn bytes_of(root: &Path, row: &str, field: &str) -> Result<u64, XtaskError> {
    let stated = register_row(root, row)?
        .get(field)
        .and_then(toml::Value::as_integer)
        .ok_or_else(|| XtaskError::Doc {
            file: REGISTER.to_owned(),
            msg: format!("[{row}] states no `{field}`"),
        })?;
    u64::try_from(stated).map_err(|_| XtaskError::Doc {
        file: REGISTER.to_owned(),
        msg: format!("[{row}].{field} is not a non-negative count: {stated}"),
    })
}

fn grouped(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len().saturating_add(digits.len() / 3));
    for (index, digit) in digits.chars().enumerate() {
        let left = digits.len().saturating_sub(index);
        if index != 0 && left % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

fn refuse_if_empty(found: usize, where_looked: &str, what: &str) -> Result<String, XtaskError> {
    if found == 0 {
        return Err(XtaskError::Doc {
            file: where_looked.to_owned(),
            msg: format!("no {what} found here; this reader is looking in the wrong place"),
        });
    }
    Ok(found.to_string())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn a_byte_count_is_grouped_every_three_digits() {
        assert_eq!(grouped(0), "0");
        assert_eq!(grouped(999), "999");
        assert_eq!(grouped(1_000), "1,000");
        assert_eq!(grouped(288_972), "288,972");
        assert_eq!(grouped(2_097_152), "2,097,152");
    }

    #[test]
    fn a_dependency_table_states_a_version_in_either_spelling() {
        let manifest: toml::Value = toml::from_str(
            "[dependencies]\n\
             toml = \"1.1\"\n\
             axum = { version = \"0.8\", features = [\"ws\"] }\n",
        )
        .unwrap();
        assert_eq!(pinned_in(&manifest, "toml").as_deref(), Some("1.1"));
        assert_eq!(pinned_in(&manifest, "axum").as_deref(), Some("0.8"));
        assert_eq!(pinned_in(&manifest, "absent"), None);
    }

    #[test]
    fn a_platform_arm_pins_as_well_as_the_package_does() {
        let manifest: toml::Value = toml::from_str(
            "[target.'cfg(windows)'.dependencies]\nwindows = { version = \"0.62\" }\n",
        )
        .unwrap();
        assert_eq!(pinned_in(&manifest, "windows").as_deref(), Some("0.62"));
    }

    #[test]
    fn the_wire_tags_are_the_snake_case_keys_the_schema_declares() {
        let commands = wire_tags("Command").unwrap();
        assert!(commands.contains("`dispatch`"), "{commands}");
        assert!(commands.contains("`put_document`"), "{commands}");
        assert_eq!(commands.matches('`').count() / 2, wire::COMMAND_NAMES.len());
        let queries = wire_tags("Query").unwrap();
        assert_eq!(queries.matches('`').count() / 2, wire::QUERY_NAMES.len());
    }
}
