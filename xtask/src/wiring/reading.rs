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

use super::{Reach, SPEC, WIRE_DIR};
use crate::report::XtaskError;
use crate::walk;

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

/// What the SPEC's reach table says, variant by variant.
pub(super) fn declared(root: &Path) -> Result<BTreeMap<String, Reach>, XtaskError> {
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
