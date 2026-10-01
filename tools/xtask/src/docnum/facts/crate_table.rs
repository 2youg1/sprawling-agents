// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The crate table a document quotes (tools/xtask/Spec.lean §8-40): one
//! row per package of the product graph, each cell taken from the place
//! that already holds it. Where a package lives and what it is called
//! come from `members`, what it may depend on from the depmap block, and
//! what it owns from its family's duty in the module map.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::lean;
use crate::members::{self, Member};
use crate::report::XtaskError;
use crate::{architecture, depmap, modmap, walk};

/// The table's head. English, because the document that quotes it is.
const HEAD: &str = "| Directory | Package | Lib | Owns | May depend on | SPEC |\n\
                    |---|---|---|---|---|---|";

/// The whole table, head first, without a trailing newline.
///
/// # Errors
/// When cargo, the depmap block or the module map cannot be read, when a
/// package is missing from the block or has no duty (`modmap::duties`),
/// and when a package has no SPEC beside it.
pub(super) fn recount(root: &Path) -> Result<String, XtaskError> {
    let product: Vec<Member> = members::members(root)?
        .into_iter()
        .filter(Member::in_product_graph)
        .collect();
    let allowed = depmap::parse_block(&walk::read_text(&root.join(architecture::PATH))?)?;
    let duties = modmap::duties(root, &product)?;
    let rows = product
        .iter()
        .zip(&duties)
        .map(|(member, duty)| row(root, member, &allowed, duty))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(std::iter::once(HEAD.to_owned())
        .chain(rows)
        .collect::<Vec<_>>()
        .join("\n"))
}

/// One package's row. The directory and the SPEC are repo-relative code
/// spans rather than links, because a link's spelling depends on the
/// directory of the document that quotes the table.
fn row(
    root: &Path,
    member: &Member,
    allowed: &BTreeMap<String, BTreeSet<String>>,
    duty: &str,
) -> Result<String, XtaskError> {
    let name = member.name();
    let edges = allowed.get(name).ok_or_else(|| XtaskError::Doc {
        file: architecture::PATH.to_owned(),
        msg: format!(
            "{name} is not in the depmap block, so the crate table cannot say what it may depend \
             on; register it in the block first"
        ),
    })?;
    let spec = if lean::migrated(root, &member.dir) {
        format!("{}/{}", member.dir, lean::ENTRY)
    } else {
        format!("{}/{name}-SPEC.md", member.dir)
    };
    let present = root
        .join(&spec)
        .try_exists()
        .map_err(|source| XtaskError::Io {
            path: spec.clone(),
            source,
        })?;
    if !present {
        return Err(XtaskError::Doc {
            file: spec,
            msg: format!(
                "no such file, so the crate table would point at nothing; run `cargo xtask spec \
                 {name}`"
            ),
        });
    }
    let lib = member
        .lib
        .as_deref()
        .map_or_else(|| "no lib".to_owned(), |lib| format!("`{lib}`"));
    let may = if edges.is_empty() {
        "nothing".to_owned()
    } else {
        edges
            .iter()
            .map(|edge| format!("`{edge}`"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    Ok(format!(
        "| `{}` | `{}` | {lib} | {} | {may} | `{spec}` |",
        member.dir,
        member.package,
        duty.replace('|', "\\|")
    ))
}
