// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


use std::path::Path;

use crate::budget;
use crate::budget::REGISTER;
use crate::report::{Violation, XtaskError};
use crate::walk;

mod mei;
pub(crate) use mei::{
    MEI_ARCHIVE_PREFIX, MEI_ROOT_PACKAGE, calyx_binary, mei_binary, mei_package,
};

pub(crate) struct Platform {
    pub(crate) suffix: &'static str,
    pub(crate) runner: &'static str,
    pub(crate) cargo_target: &'static str,
    pub(crate) package: &'static str,
    pub(crate) os: &'static str,
    pub(crate) cpu: &'static str,
    pub(crate) binary: &'static str,
}

pub(crate) const ROOT_PACKAGE: &str = "sprawling";

pub(crate) const PLATFORMS: [Platform; 3] = [
    Platform {
        suffix: "-windows-x86_64.zip",
        runner: "windows-latest",
        cargo_target: "",
        package: "@sprawling/sprawling-windows-x64",
        os: "win32",
        cpu: "x64",
        binary: "sprawling.exe",
    },
    Platform {
        suffix: "-macos-aarch64.zip",
        runner: "macos-latest",
        cargo_target: "",
        package: "@sprawling/sprawling-darwin-arm64",
        os: "darwin",
        cpu: "arm64",
        binary: "sprawling",
    },
    Platform {
        suffix: "-x86_64-unknown-linux-musl.zip",
        runner: "ubuntu-latest",
        cargo_target: "x86_64-unknown-linux-musl",
        package: "@sprawling/sprawling-linux-x64-musl",
        os: "linux",
        cpu: "x64",
        binary: "sprawling",
    },
];

pub(crate) fn with_suffix(suffix: &str) -> Option<&'static Platform> {
    PLATFORMS.iter().find(|row| row.suffix == suffix)
}

pub(crate) const WORKFLOW: &str = ".github/workflows/release.yml";

const SHIM: &str = "crates/xtask/src/channel/shim.js";

const SPELLERS: [&str; 4] = [
    WORKFLOW,
    "install.sh",
    "install.ps1",
    "crates/sprawling/Cargo.toml",
];

const REGISTER_ROW: &str = "archive_naming";

pub(crate) fn restated(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = matrix_rows(root)?;
    violations.extend(shim_rows(root)?);
    violations.extend(spelled_suffixes(root)?);
    Ok(violations)
}

fn matrix(text: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let mut out = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let Some(runner) = line.strip_prefix("- os:") else {
            continue;
        };
        let target = lines
            .get(index.saturating_add(1))
            .and_then(|next| next.strip_prefix("target:"))
            .unwrap_or_default();
        out.push((unquoted(runner), unquoted(target)));
    }
    out
}

fn unquoted(value: &str) -> String {
    value
        .trim()
        .trim_end_matches(',')
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_owned()
}

fn matrix_rows(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let built = matrix(&walk::read_text(&root.join(WORKFLOW))?);
    let wanted: Vec<(String, String)> = PLATFORMS
        .iter()
        .map(|row| (row.runner.to_owned(), row.cargo_target.to_owned()))
        .collect();
    let mut out = Vec::new();
    for (runner, target) in &wanted {
        if !built.contains(&(runner.clone(), target.clone())) {
            out.push(disagrees(
                WORKFLOW,
                format!("no matrix row builds `{runner}` for target `{target}`"),
            ));
        }
    }
    for pair in &built {
        if !wanted.contains(pair) {
            out.push(disagrees(
                WORKFLOW,
                format!(
                    "a matrix row builds `{}` for target `{}`, which no platform claims",
                    pair.0, pair.1
                ),
            ));
        }
    }
    Ok(out)
}

fn shim(text: &str) -> Vec<(String, String, String)> {
    shim_table(text, "PLATFORMS = {")
}

fn mei_shim(text: &str) -> Vec<(String, String, String, String)> {
    let Some(body) = object_body(text, "MEI_PLATFORMS = {") else {
        return Vec::new();
    };
    entries(body).into_iter().filter_map(mei_entry).collect()
}

fn shim_table(text: &str, header: &str) -> Vec<(String, String, String)> {
    let Some(body) = object_body(text, header) else {
        return Vec::new();
    };
    entries(body).into_iter().filter_map(entry).collect()
}

fn object_body<'a>(text: &'a str, header: &str) -> Option<&'a str> {
    let body = text.split_once(header)?.1;
    let mut depth = 1_usize;
    let mut quoted = false;
    let mut escaped = false;
    for (at, mark) in body.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match mark {
            '\\' if quoted => escaped = true,
            '"' => quoted = !quoted,
            '{' if !quoted => depth = depth.saturating_add(1),
            '}' if !quoted => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return body.get(..at);
                }
            }
            _ => {}
        }
    }
    None
}

fn entries(body: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut from = 0_usize;
    let mut depth = 0_usize;
    let mut quoted = false;
    let mut escaped = false;
    for (at, mark) in body.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match mark {
            '\\' if quoted => escaped = true,
            '"' => quoted = !quoted,
            '{' if !quoted => depth = depth.saturating_add(1),
            '}' if !quoted => depth = depth.saturating_sub(1),
            ',' if !quoted && depth == 0 => {
                out.extend(body.get(from..at));
                from = at.saturating_add(1);
            }
            _ => {}
        }
    }
    if let Some(last) = body.get(from..)
        && !last.trim().is_empty()
    {
        out.push(last);
    }
    out
}

fn entry(entry: &str) -> Option<(String, String, String)> {
    let key = entry.trim_start().strip_prefix('"')?;
    let (key, _) = key.split_once('"')?;
    let package = field(entry, "package:")?;
    let binary = field(entry, "binary:")?;
    Some((key.to_owned(), package, binary))
}

fn mei_entry(entry: &str) -> Option<(String, String, String, String)> {
    let key = entry.trim_start().strip_prefix('"')?;
    let (key, _) = key.split_once('"')?;
    let package = field(entry, "package:")?;
    let core = field(entry, "core:")?;
    let face = field(entry, "face:")?;
    Some((key.to_owned(), package, core, face))
}

fn field(text: &str, key: &str) -> Option<String> {
    let after = text.split_once(key)?.1.trim_start();
    let inner = after.strip_prefix('"')?;
    let (value, _) = inner.split_once('"')?;
    Some(value.to_owned())
}

fn shim_rows(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let text = walk::read_text(&root.join(SHIM))?;
    let held = shim(&text);
    let wanted: Vec<(String, String, String)> = PLATFORMS
        .iter()
        .map(|row| {
            (
                format!("{} {}", row.os, row.cpu),
                row.package.to_owned(),
                row.binary.to_owned(),
            )
        })
        .collect();
    let mut out = Vec::new();
    for row in &wanted {
        if !held.contains(row) {
            out.push(disagrees(
                SHIM,
                format!("no entry maps `{}` to `{}` and `{}`", row.0, row.1, row.2),
            ));
        }
    }
    for row in &held {
        if !wanted.contains(row) {
            out.push(disagrees(
                SHIM,
                format!("`{}` resolves `{}`, which no platform claims", row.0, row.1),
            ));
        }
    }

    let held = mei_shim(&text);
    let wanted: Vec<(String, String, String, String)> = PLATFORMS
        .iter()
        .filter_map(|row| {
            mei_package(row).map(|package| {
                (
                    format!("{} {}", row.os, row.cpu),
                    package.to_owned(),
                    mei_binary(row).to_owned(),
                    calyx_binary(row).to_owned(),
                )
            })
        })
        .collect();
    for row in &wanted {
        if !held.contains(row) {
            out.push(disagrees(
                SHIM,
                format!(
                    "no Mei entry maps `{}` to `{}`, core `{}`, face `{}`",
                    row.0, row.1, row.2, row.3
                ),
            ));
        }
    }
    for row in &held {
        if !wanted.contains(row) {
            out.push(disagrees(
                SHIM,
                format!(
                    "Mei maps `{}` to `{}`, which is not a supported Mei package",
                    row.0, row.1
                ),
            ));
        }
    }
    if !text.contains(&format!("packageManifest.name === \"{MEI_ROOT_PACKAGE}\"")) {
        out.push(disagrees(
            SHIM,
            format!("the product selector does not recognise `{MEI_ROOT_PACKAGE}`"),
        ));
    }
    Ok(out)
}

fn spelled_suffixes(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let recorded = registered(root)?;
    let mut out = Vec::new();
    let mut met: Vec<String> = Vec::new();
    for rel in SPELLERS {
        let path = root.join(rel);
        if !path.is_file() {
            continue;
        }
        for (number, line) in walk::read_text(&path)?.lines().enumerate() {
            for suffix in suffixes(line) {
                met.push(suffix.clone());
                if PLATFORMS.iter().any(|row| row.suffix == suffix) {
                    continue;
                }
                if recorded.contains(&suffix) {
                    continue;
                }
                out.push(disagrees(
                    &format!("{rel}:{}", number.saturating_add(1)),
                    format!("`{suffix}` ends no archive this release builds"),
                ));
            }
        }
    }
    out.extend(spent_suffixes(&recorded, &met));
    Ok(out)
}

fn spent_suffixes(recorded: &[String], met: &[String]) -> Vec<Violation> {
    let mut out = Vec::new();
    for suffix in recorded {
        let violation = if PLATFORMS.iter().any(|row| row.suffix == *suffix) {
            format!("`{suffix}` is an archive this release now builds")
        } else if !met.contains(suffix) {
            format!("`{suffix}` is recorded and nobody spells it")
        } else {
            continue;
        };
        out.push(Violation {
            gate: "artifact",
            location: format!("{REGISTER} [{REGISTER_ROW}.predating]"),
            rule: "an exception that is no longer needed is struck from the register".to_owned(),
            violation,
            alternative: "delete its row: a spent exception left in place re-excuses whatever \
                          is written at that address next"
                .to_owned(),
        });
    }
    out
}

fn registered(root: &Path) -> Result<Vec<String>, XtaskError> {
    let register = budget::register(root)?;
    let Some(listed) = register
        .get(REGISTER_ROW)
        .and_then(|row| row.get("predating"))
        .and_then(|table| table.get("suffixes"))
        .and_then(toml::Value::as_array)
    else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for value in listed {
        let suffix = value.as_str().ok_or_else(|| XtaskError::Doc {
            file: REGISTER.to_owned(),
            msg: format!("{REGISTER_ROW}.predating holds something that is not a suffix"),
        })?;
        out.push(suffix.to_owned());
    }
    Ok(out)
}

fn suffixes(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in line.chars().chain(std::iter::once(' ')) {
        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-') {
            current.push(ch);
            continue;
        }
        let token = std::mem::take(&mut current);
        if token.starts_with('-') && token.ends_with(".zip") {
            out.push(token);
        }
    }
    out
}

fn disagrees(location: &str, violation: String) -> Violation {
    Violation {
        gate: "artifact",
        location: location.to_owned(),
        rule: "every platform a release builds for is spelled the way `xtask::platform` \
               spells it"
            .to_owned(),
        violation,
        alternative:
            "bring this file to the table in `crates/xtask/src/platform.rs`, or change that \
                      table and every file that restates it in one change-set"
                .to_owned(),
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
