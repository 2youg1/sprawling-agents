// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;
use std::path::Path;

use crate::report::{Violation, XtaskError};

mod emit;

const TARGET_DIR: &str = concat!(crate::walk::client_src!(), "/wire");
const LEGACY_FILE: &str = concat!(crate::walk::client_src!(), "/wire.ts");

pub(super) struct Constants {
    pub(super) wire_v: u32,
    pub(super) hash: String,

    pub(super) city_run: String,

    pub(super) body_px_min: u32,

    pub(super) line_height_min: u32,
    pub(super) tracking_min: i32,

    pub(super) default_effort: &'static str,

    pub(super) slash: Vec<[&'static str; 4]>,
}

fn render() -> Result<emit::Bundle, XtaskError> {
    let constants = Constants {
        wire_v: wire::WIRE_V,
        hash: wire::schema_hash().to_string(),
        city_run: kernel::RunId::CITY.to_string(),
        body_px_min: wire::BODY_PX_MIN,
        line_height_min: wire::LINE_HEIGHT_MIN,
        tracking_min: wire::TRACKING_MIN,
        default_effort: kernel::consts_policy::DEFAULT_EFFORT.as_str(),
        slash: wire::Slash::ALL
            .into_iter()
            .map(|verb| {
                [
                    verb.spelling(),
                    verb.takes(),
                    verb.offered().as_str(),
                    verb.says(),
                ]
            })
            .collect(),
    };
    emit::emit(&wire::wire_schema(), &constants).map_err(|refused| XtaskError::Doc {
        file: format!("wire::wire_schema at {}", refused.at),
        msg: refused.why,
    })
}

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let expected = render()?;
    let mut violations = Vec::new();
    let legacy = root.join(LEGACY_FILE);
    if legacy.is_file() {
        violations.push(Violation {
            gate: "wire-ts",
            location: LEGACY_FILE.to_owned(),
            rule: "the client's wire types live under RefRain/src/wire/".to_owned(),
            violation: "RefRain/src/wire.ts is still present".to_owned(),
            alternative: "run `cargo xtask wire-ts --write` and commit the result".to_owned(),
        });
    }
    let mut expected_paths = BTreeSet::new();
    for (rel, body) in &expected.files {
        let path = root.join(crate::walk::client_src!()).join(rel);
        expected_paths.insert(rel.clone());
        let actual = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(source) => {
                return Err(XtaskError::Io {
                    path: path.display().to_string(),
                    source,
                });
            }
        };
        let Some((line, wanted, found)) = first_difference(body, &actual) else {
            continue;
        };
        let location = format!("{}/{rel}:{line}", crate::walk::client_src!());
        violations.push(Violation {
            gate: "wire-ts",
            location,
            rule: "the client's wire types are the ones the Rust wire generates".to_owned(),
            violation: if actual.is_empty() {
                "the file is absent".to_owned()
            } else {
                format!("line {line} reads `{found}` where the wire says `{wanted}`")
            },
            alternative: "run `cargo xtask wire-ts --write` and commit the result".to_owned(),
        });
    }
    let dir = root.join(TARGET_DIR);
    if dir.is_dir() {
        for file in crate::walk::files_with_ext(&dir, &["ts"])? {
            let rel = crate::walk::rel(root, &file);
            let Some(under) = rel.strip_prefix(&format!("{}/", crate::walk::client_src!())) else {
                continue;
            };
            if !expected_paths.contains(under) {
                violations.push(Violation {
                    gate: "wire-ts",
                    location: rel,
                    rule: "the client's wire types are the ones the Rust wire generates".to_owned(),
                    violation: "this file is not part of the generated set".to_owned(),
                    alternative: "run `cargo xtask wire-ts --write` and commit the result"
                        .to_owned(),
                });
            }
        }
    }
    Ok(violations)
}

pub(crate) fn write(root: &Path) -> Result<String, XtaskError> {
    let bundle = render()?;
    let dir = root.join(TARGET_DIR);
    std::fs::create_dir_all(&dir).map_err(|source| XtaskError::Io {
        path: dir.display().to_string(),
        source,
    })?;
    let legacy = root.join(LEGACY_FILE);
    if legacy.is_file() {
        std::fs::remove_file(&legacy).map_err(|source| XtaskError::Io {
            path: legacy.display().to_string(),
            source,
        })?;
    }
    let mut kept = BTreeSet::new();
    let mut written = Vec::new();
    for (rel, body) in &bundle.files {
        let path = root.join(crate::walk::client_src!()).join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| XtaskError::Io {
                path: parent.display().to_string(),
                source,
            })?;
        }
        std::fs::write(&path, body).map_err(|source| XtaskError::Io {
            path: path.display().to_string(),
            source,
        })?;
        kept.insert(rel.clone());
        written.push(format!("{}/{}", crate::walk::client_src!(), rel));
    }
    if dir.is_dir() {
        for file in crate::walk::files_with_ext(&dir, &["ts"])? {
            let rel = crate::walk::rel(root, &file);
            let Some(under) = rel.strip_prefix(&format!("{}/", crate::walk::client_src!())) else {
                continue;
            };
            if !kept.contains(under) {
                std::fs::remove_file(&file).map_err(|source| XtaskError::Io {
                    path: file.display().to_string(),
                    source,
                })?;
            }
        }
    }
    Ok(format!("written: {}\n", written.join(", ")))
}

fn first_difference(expected: &str, actual: &str) -> Option<(usize, String, String)> {
    let mut wanted = expected.lines();
    let mut found = actual.lines();
    let mut line = 0_usize;
    loop {
        line = line.saturating_add(1);
        match (wanted.next(), found.next()) {
            (None, None) => return None,
            (w, f) if w == f => {}
            (w, f) => {
                return Some((
                    line,
                    w.unwrap_or_default().to_owned(),
                    f.unwrap_or_default().to_owned(),
                ));
            }
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests;
