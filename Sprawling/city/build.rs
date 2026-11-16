// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

const SKILLS_DIR: &str = "skills";
const SKILL_FILE: &str = "SKILL.md";
const LICENCE_FILE: &str = "LICENSES.md";
const TABLE: &str = "shipped_skills.rs";

fn main() {
    if let Err(msg) = embed() {
        println!("cargo::error=shipped skill embed failed: {msg}");
    }
}

fn embed() -> Result<(), String> {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").map_err(|err| err.to_string())?;
    let out_dir = std::env::var("OUT_DIR").map_err(|err| err.to_string())?;
    let skills = Path::new(&manifest).join(SKILLS_DIR);
    println!("cargo::rerun-if-changed={}", skills.display());
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    for skill in listed(&skills)? {
        if skill.join(SKILL_FILE).is_file() {
            collect(&skills, &skill, &mut files)?;
        }
    }
    let licence = skills.join(LICENCE_FILE);
    if !licence.is_file() {
        return Err(format!(
            "{} is missing, and the shipped skills cannot carry the notices they owe",
            licence.display()
        ));
    }
    files.push((LICENCE_FILE.to_owned(), licence));
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut table = String::from("&[\n");
    for (path, file) in &files {
        let absolute = file.to_string_lossy();
        table.push_str(&format!("    ({path:?}, include_bytes!({absolute:?})),\n"));
    }
    table.push_str("]\n");
    std::fs::write(Path::new(&out_dir).join(TABLE), table).map_err(|err| err.to_string())
}

fn listed(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|err| format!("{}: {err}", dir.display()))? {
        found.push(entry.map_err(|err| err.to_string())?.path());
    }
    Ok(found)
}

fn collect(root: &Path, dir: &Path, files: &mut Vec<(String, PathBuf)>) -> Result<(), String> {
    for path in listed(dir)? {
        if path.is_dir() {
            collect(root, &path, files)?;
        } else {
            let relative = path
                .strip_prefix(root)
                .map_err(|err| err.to_string())?
                .components()
                .map(|segment| segment.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            files.push((relative, path));
        }
    }
    Ok(())
}
