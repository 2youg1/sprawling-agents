// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::{analysis, error::BenchError, manifest, observation, report as rendering};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Computed {

    pub exports: String,

    pub markdown: String,
}

pub fn export(round: &Path) -> Result<Computed, BenchError> {
    let manifest_text = read_string(&round.join("manifest.json"))?;
    let manifest = manifest::Manifest::parse(&manifest_text)?;
    let csv_path = round.join("results.csv");
    let csv = read_bytes(&csv_path)?;
    let actual = hex_digest(&csv);
    let expected = manifest.input_hashes.get("results.csv").ok_or_else(|| {
        BenchError::input(
            "verify results.csv digest",
            "input_hashes.results.csv is missing".to_owned(),
            "pin the CSV's SHA-256 in manifest.json",
        )
    })?;
    if &actual != expected {
        return Err(BenchError::input(
            "verify results.csv digest",
            format!("expected {expected}, found {actual}"),
            "restore the pinned observations or pre-register a new round and digest",
        ));
    }
    let observations = observation::parse(&csv)?;
    let value = analysis::analyze(&manifest, &observations)?;
    let exports = rendering::canonical(&value)?;
    let markdown = rendering::markdown(&value)?;
    Ok(Computed { exports, markdown })
}

pub fn export_write(round: &Path) -> Result<Computed, BenchError> {
    let computed = export(round)?;
    let readme_path = round.join("README.md");
    let readme = read_string(&readme_path)?;
    let updated = rendering::replace_generated(&readme, &computed.markdown)?;
    write_atomically(&round.join("exports.json"), computed.exports.as_bytes())?;
    write_atomically(&readme_path, updated.as_bytes())?;
    Ok(computed)
}

pub fn verify(round: &Path) -> Result<Computed, BenchError> {
    let computed = export(round)?;
    let published = read_string(&round.join("exports.json"))?;
    if published != computed.exports {
        return Err(BenchError::mismatch(
            "exports.json differs from the fresh recomputation".to_owned(),
        ));
    }
    let readme = read_string(&round.join("README.md"))?;
    let expected = rendering::replace_generated(&readme, &computed.markdown)?;
    if readme != expected {
        return Err(BenchError::mismatch(
            "README.md's generated section differs from the exports".to_owned(),
        ));
    }
    Ok(computed)
}

pub fn report(round: &Path, write: bool) -> Result<Computed, BenchError> {
    let exports = read_string(&round.join("exports.json"))?;
    let value: serde_json::Value = serde_json::from_str(&exports).map_err(|error| {
        BenchError::input(
            "parse exports.json",
            error.to_string(),
            "regenerate exports.json with `benchstat export <round> --write`",
        )
    })?;
    let markdown = rendering::markdown(&value)?;
    if write {
        let readme_path = round.join("README.md");
        let readme = read_string(&readme_path)?;
        let updated = rendering::replace_generated(&readme, &markdown)?;
        write_atomically(&readme_path, updated.as_bytes())?;
    }
    Ok(Computed { exports, markdown })
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, BenchError> {
    std::fs::read(path).map_err(|error| BenchError::io("read file", display(path), error))
}

fn read_string(path: &Path) -> Result<String, BenchError> {
    std::fs::read_to_string(path)
        .map_err(|error| BenchError::io("read UTF-8 file", display(path), error))
}

fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), BenchError> {
    let part = temporary(path, "part");
    let backup = temporary(path, "backup");
    if part.exists() || backup.exists() {
        return Err(BenchError::input(
            "prepare atomic report write",
            format!("stale temporary file beside {}", display(path)),
            "inspect and remove the named .part or .backup file before retrying",
        ));
    }
    std::fs::write(&part, bytes)
        .map_err(|error| BenchError::io("write temporary report", display(&part), error))?;
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(&part)
        .map_err(|error| BenchError::io("open temporary report", display(&part), error))?;
    file.sync_all()
        .map_err(|error| BenchError::io("sync temporary report", display(&part), error))?;
    let existed = path.exists();
    if existed {
        std::fs::rename(path, &backup)
            .map_err(|error| BenchError::io("preserve previous report", display(&backup), error))?;
    }
    if let Err(error) = std::fs::rename(&part, path) {
        if existed {
            std::fs::rename(&backup, path).map_err(|rollback| {
                BenchError::io(
                    "restore previous report after failed publication",
                    format!(
                        "{}: rename failed: {error}; rollback failed: {rollback}",
                        display(path)
                    ),
                    rollback,
                )
            })?;
        }
        return Err(BenchError::io("publish report", display(path), error));
    }
    if existed {
        std::fs::remove_file(&backup)
            .map_err(|error| BenchError::io("remove report backup", display(&backup), error))?;
    }
    Ok(())
}

fn temporary(path: &Path, extension: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(format!(".benchstat-{extension}"));
    PathBuf::from(name)
}

fn display(path: &Path) -> String {
    path.display().to_string()
}

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
