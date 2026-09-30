// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Reading the two declarations this gate compares.
//!
//! One is Rust and one is a Markdown table, and neither is written for
//! a machine to read; both readers therefore fail loudly rather than
//! return an empty list, because a gate that silently reads nothing
//! passes everything.

use std::collections::BTreeMap;
use std::path::Path;

use super::{Reach, SPEC, WIRE_CRATE, WIRE_DIR};
use crate::lean;
use crate::report::XtaskError;
use crate::walk;

/// The `def` a migrated wire specification writes the reach table as.
const REACH: &str = "Command.reach";

/// The variants of an enum, read out of the source that declares it.
///
/// Parsed rather than pattern-matched line by line, for the reason
/// `length` gives: a brace is not always a block, and a gate that reads
/// its own input wrongly produces a list of offenders that is wrong in
/// every entry.
pub(super) fn variants(root: &Path, enum_name: &str) -> Result<Vec<String>, XtaskError> {
    let dir = root.join(WIRE_DIR);
    let mut sources: Vec<std::path::PathBuf> = walk::files_with_ext(&dir, &["rs"])?;
    sources.sort();
    for source in sources {
        let text = walk::read_text(&source)?;
        let parsed = syn::parse_file(&text).map_err(|err| XtaskError::Doc {
            file: walk::rel(root, &source),
            msg: format!("this file does not parse as Rust: {err}"),
        })?;
        for item in &parsed.items {
            let declared = match item {
                syn::Item::Enum(found) => found.clone(),
                // `named_frames!` wraps the declaration so the variant
                // list and the name table cannot disagree; the enum is
                // still written out in full inside it.
                syn::Item::Macro(wrapper) => {
                    let leading = |input: syn::parse::ParseStream<'_>| {
                        let found: syn::ItemEnum = input.parse()?;
                        let _rest: proc_macro2::TokenStream = input.parse()?;
                        Ok(found)
                    };
                    match syn::parse::Parser::parse2(leading, wrapper.mac.tokens.clone()) {
                        Ok(found) => found,
                        Err(_) => continue,
                    }
                }
                _ => continue,
            };
            if declared.ident == enum_name {
                return Ok(declared
                    .variants
                    .iter()
                    .map(|found| found.ident.to_string())
                    .collect());
            }
        }
    }
    Err(XtaskError::Doc {
        file: WIRE_DIR.to_owned(),
        msg: format!("no module here declares `enum {enum_name}`"),
    })
}

/// What the SPEC's reach table says, variant by variant: the arms of
/// `def Command.reach` once the wire crate's specification is Lean, the
/// Markdown table until then.
pub(super) fn declared(root: &Path) -> Result<BTreeMap<String, Reach>, XtaskError> {
    markdown_reach(root)
}

/// The arms of `def Command.reach`, each right-hand side one of the four
/// reaches with a leading dot (xtask-SPEC.md section 8-43).
///
/// # Errors
/// When no file holds the `def`, or an arm answers something that is not
/// a reach: either would otherwise pass every verb unclassified.
fn lean_reach(sources: &[lean::Source]) -> Result<BTreeMap<String, Reach>, XtaskError> {
    let (path, table) = sources
        .iter()
        .find_map(|source| {
            lean::arms(&lean::code(&source.text), REACH).map(|table| (&source.path, table))
        })
        .ok_or_else(|| XtaskError::Doc {
            file: WIRE_CRATE.to_owned(),
            msg: format!(
                "no `def {REACH}` written one arm per line in the Lean specification; write it, \
                 so every verb has a reach"
            ),
        })?;
    table
        .arms
        .into_iter()
        .map(|arm| {
            let reach = arm
                .value
                .strip_prefix('.')
                .and_then(Reach::parse)
                .ok_or_else(|| XtaskError::Doc {
                    file: format!("{path}:{}", arm.line),
                    msg: format!(
                        "`{}` answers `{}`, which is not one of .client, .push, .handshake, \
                         .sealed",
                        arm.pattern, arm.value
                    ),
                })?;
            Ok((arm.pattern, reach))
        })
        .collect()
}

/// The reach table of the Markdown SPEC.
fn markdown_reach(root: &Path) -> Result<BTreeMap<String, Reach>, XtaskError> {
    let text = walk::read_text(&root.join(SPEC))?;
    let mut out = BTreeMap::new();
    for line in text.lines() {
        let Some(cells) = cells(line) else {
            continue;
        };
        let (Some(first), Some(second)) = (cells.first(), cells.get(1)) else {
            continue;
        };
        let Some(name) = first.strip_prefix('`').and_then(|c| c.strip_suffix('`')) else {
            continue;
        };
        // A row whose reach cell is not one of the four is not a reach
        // row: the same document holds other tables with a backticked
        // first column, and reading them would invent verbs.
        let Some(reach) = Reach::parse(second) else {
            continue;
        };
        out.insert(name.to_owned(), reach);
    }
    Ok(out)
}

fn cells(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix('|')?.strip_suffix('|')?;
    Some(
        inner
            .split('|')
            .map(|cell| cell.trim().to_owned())
            .collect(),
    )
}
