// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::path::Path;

use super::{Reach, WIRE_CRATE, WIRE_DIR};
use crate::lean;
use crate::report::XtaskError;
use crate::walk;

const REACH: &str = "Command.reach";

pub(super) struct ReachTable {
    pub(super) path: String,
    pub(super) rows: BTreeMap<String, Reach>,
}

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

pub(super) fn declared(root: &Path) -> Result<ReachTable, XtaskError> {
    let sources = specification(root)?;
    lean_reach(&sources)
}

pub(super) fn specification(root: &Path) -> Result<Vec<lean::Source>, XtaskError> {
    lean::specification(root, WIRE_CRATE)?.ok_or_else(|| XtaskError::Doc {
        file: WIRE_CRATE.to_owned(),
        msg: format!(
            "no {} here, and the reach and class of every verb are arms of it; restore the \
             wire crate's specification",
            lean::ENTRY
        ),
    })
}

fn lean_reach(sources: &[lean::Source]) -> Result<ReachTable, XtaskError> {
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
    let rows = table
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
        .collect::<Result<_, XtaskError>>()?;
    Ok(ReachTable {
        path: path.clone(),
        rows,
    })
}
