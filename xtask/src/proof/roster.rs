// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Reading the roster out of the source that declares it.
//!
//! Everything here answers one question: which `#[kani::proof]`
//! functions exist, what are they called, and which of them carry a
//! reason for not being proved. The gate and the two commands live in
//! the parent module and take this answer as given.

use std::path::Path;

use super::{ATTRIBUTE, EXCUSED, Harness};
use crate::report::XtaskError;
use crate::walk;

/// The package directories under `crates/`, sorted.
pub(super) fn crates(root: &Path) -> Result<Vec<std::path::PathBuf>, XtaskError> {
    let dir = root.join("crates");
    let mut out = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|source| XtaskError::Io {
        path: dir.display().to_string(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| XtaskError::Io {
            path: dir.display().to_string(),
            source,
        })?;
        let path = entry.path();
        if path.join("Cargo.toml").is_file() {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

/// A package's name as its own manifest gives it, not as its directory
/// happens to be spelled: `cargo kani -p` takes the former.
pub(super) fn package_name(dir: &Path) -> Result<String, XtaskError> {
    let manifest = dir.join("Cargo.toml");
    let text = walk::read_text(&manifest)?;
    for line in text.lines() {
        let Some(value) = line.strip_prefix("name = ") else {
            continue;
        };
        return Ok(value.trim().trim_matches('"').to_owned());
    }
    Err(XtaskError::Doc {
        file: walk::rel(dir, &manifest),
        msg: "no `name = ` line; a package without a name cannot be proved".to_owned(),
    })
}

/// The module path a file contributes, from its path inside `src/`.
///
/// The crate root contributes nothing; a leaf file and a directory's
/// own `mod` file both contribute that name. A binary root is not a
/// module path at all.
pub(super) fn module_base(rel: &str) -> Option<String> {
    let stem = rel.strip_suffix(".rs")?;
    match stem {
        "lib" => Some(String::new()),
        "main" => None,
        _ => {
            let trimmed = stem.strip_suffix("/mod").unwrap_or(stem);
            Some(trimmed.replace('/', "::"))
        }
    }
}

/// Every harness one file declares, with its inline module nesting
/// resolved: the ones in this tree all sit in `#[cfg(kani)] mod
/// verification`, and that segment is part of the name kani matches.
pub(super) fn in_file(text: &str, package: &str, rel: &str, base: &str) -> Vec<Harness> {
    let mut out = Vec::new();
    let mut stack: Vec<(String, usize)> = Vec::new();
    let mut depth = 0_usize;
    let mut pending: Option<String> = None;
    let mut in_block = false;
    let mut armed: Option<(usize, Option<String>)> = None;
    let mut marker: Option<String> = None;
    for (index, raw) in text.lines().enumerate() {
        let trimmed = raw.trim();
        if trimmed == ATTRIBUTE {
            armed = Some((index, marker.clone()));
        }
        marker = trimmed
            .strip_prefix(EXCUSED)
            .map(|reason| reason.trim().to_owned());
        let code = strip(raw, &mut in_block);
        if let Some(name) = declared_module(&code) {
            pending = Some(name);
        }
        if let Some(name) = declared_fn(&code)
            && let Some((line, excuse)) = armed.take()
        {
            let mut full = String::from(base);
            for (segment, _) in &stack {
                push_segment(&mut full, segment);
            }
            push_segment(&mut full, &name);
            out.push(Harness {
                package: package.to_owned(),
                location: format!("{rel}:{}", line.saturating_add(1)),
                name: full,
                excuse,
            });
        }
        for symbol in code.chars() {
            match symbol {
                '{' => {
                    depth = depth.saturating_add(1);
                    if let Some(name) = pending.take() {
                        stack.push((name, depth));
                    }
                }
                '}' => {
                    if stack.last().is_some_and(|(_, at)| *at == depth) {
                        stack.pop();
                    }
                    depth = depth.saturating_sub(1);
                }
                ';' => pending = None,
                _ => {}
            }
        }
    }
    out
}

pub(super) fn push_segment(path: &mut String, segment: &str) {
    if !path.is_empty() {
        path.push_str("::");
    }
    path.push_str(segment);
}

/// The module a line opens, if it opens one.
pub(super) fn declared_module(code: &str) -> Option<String> {
    identifier_after(code, "mod")
}

/// The function a line declares, if it declares one.
pub(super) fn declared_fn(code: &str) -> Option<String> {
    identifier_after(code, "fn")
}

/// The identifier following a keyword used as a whole word.
pub(super) fn identifier_after(code: &str, keyword: &str) -> Option<String> {
    let mut rest = code;
    loop {
        let at = rest.find(keyword)?;
        let before = rest.get(..at).and_then(|head| head.chars().next_back());
        let after = rest.get(at.saturating_add(keyword.len())..)?;
        let boundary = before.is_none_or(|c| !c.is_alphanumeric() && c != '_');
        if boundary && after.starts_with(' ') {
            let name: String = after
                .trim_start()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                return Some(name);
            }
        }
        rest = after;
    }
}

/// A line with its comments and string literals removed, so brace
/// counting reads code and not prose. `in_block` carries `/* */` across
/// lines.
pub(super) fn strip(line: &str, in_block: &mut bool) -> String {
    let mut out = String::new();
    let mut chars = line.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;
    while let Some(symbol) = chars.next() {
        if *in_block {
            if symbol == '*' && chars.peek() == Some(&'/') {
                chars.next();
                *in_block = false;
            }
            continue;
        }
        if in_string {
            if escaped {
                escaped = false;
            } else if symbol == '\\' {
                escaped = true;
            } else if symbol == '"' {
                in_string = false;
            }
            continue;
        }
        match symbol {
            '/' if chars.peek() == Some(&'/') => break,
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                *in_block = true;
            }
            '"' => in_string = true,
            '\'' => {
                // A char literal, or a lifetime. Only the literal can
                // hold a brace, and it is at most three more characters.
                let mut ahead = chars.clone();
                match ahead.next() {
                    Some('\\') => {
                        chars.next();
                        chars.next();
                        chars.next();
                    }
                    Some(_) if ahead.next() == Some('\'') => {
                        chars.next();
                        chars.next();
                    }
                    _ => {}
                }
            }
            _ => out.push(symbol),
        }
    }
    out
}
