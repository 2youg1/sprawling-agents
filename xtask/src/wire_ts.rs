// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `cargo xtask wire-ts [--write]`: the TypeScript client's wire types,
//! generated from the Rust wire so both ends of the socket have one
//! authority (channels-SPEC.md section 8-16).
//!
//! Without `--write` this is a comparison: the file on disk against the
//! text the wire produces now, refused at the first line that differs.
//! With `--write` it is the only file this command ever writes. Same
//! shape as `apisync` and `badge`: a gate that generates its own record
//! and then holds the tree to it.

use std::path::Path;

use crate::report::{Violation, XtaskError};

mod emit;

/// Where the generated file lives, relative to the repository root.
const TARGET: &str = "client/src/wire.ts";

/// The text the wire produces now.
fn render() -> Result<String, XtaskError> {
    let hash = channels::schema_hash().to_string();
    emit::emit(&channels::wire_schema(), channels::WIRE_V, &hash).map_err(|refused| {
        XtaskError::Doc {
            file: format!("channels::wire_schema at {}", refused.at),
            msg: refused.why,
        }
    })
}

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let expected = render()?;
    let path = root.join(TARGET);
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
    let Some((line, wanted, found)) = first_difference(&expected, &actual) else {
        return Ok(Vec::new());
    };
    Ok(vec![Violation {
        gate: "wire-ts",
        location: format!("{TARGET}:{line}"),
        rule: "the client's wire types are the ones the Rust wire generates".to_owned(),
        violation: if actual.is_empty() {
            "the file is absent".to_owned()
        } else {
            format!("line {line} reads `{found}` where the wire says `{wanted}`")
        },
        alternative: "run `cargo xtask wire-ts --write` and commit the result".to_owned(),
    }])
}

/// `cargo xtask wire-ts --write`: the one file this command writes.
pub(crate) fn write(root: &Path) -> Result<String, XtaskError> {
    let text = render()?;
    let path = root.join(TARGET);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|source| XtaskError::Io {
            path: dir.display().to_string(),
            source,
        })?;
    }
    std::fs::write(&path, text).map_err(|source| XtaskError::Io {
        path: path.display().to_string(),
        source,
    })?;
    Ok(format!("written: {TARGET}\n"))
}

/// The first line where two texts part: its one-based number, what was
/// expected there, and what was found. A text that ends early is found
/// to have an empty line where the other goes on.
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
