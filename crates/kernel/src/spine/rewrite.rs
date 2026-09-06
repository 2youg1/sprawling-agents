// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Table rewrites: status changes and child insertions.

use crate::error::{AxCode, AxError};
use crate::locator::Locator;
use crate::node_id::NodeId;

use super::grammar::{body_row, check_roadmap_shape, locate_table, split_table_row};
use super::row::{NewChild, ROADMAP_COLUMNS, RoadmapRow, RoadmapShape, RoadmapStatus};

/// One row as the table writes it. The normal form: the same edit twice
/// renders the same bytes.
fn draw_row(row: &RoadmapRow, status: RoadmapStatus, evidence: Option<&Locator>) -> String {
    let needs = row
        .needs
        .iter()
        .map(NodeId::to_string)
        .collect::<Vec<String>>()
        .join(", ");
    let cell = evidence.map_or_else(String::new, Locator::to_string);
    format!(
        "| {} | {} | {} | {needs} | {} | {cell} |",
        row.id,
        row.item,
        row.weight,
        status.spelling()
    )
}

/// Rewrites one row of the first table, returning the whole text.
///
/// This is the table's only status-editing entrance.
///
/// # Errors
/// Refuses a text whose table does not parse, an index no row carries,
/// and a `Done` without evidence — that last one is exactly the row
/// `crate::plan` declines to count, and a writer that can produce it is
/// a writer that can make a plan look finished while the figure stands
/// still.
pub fn set_roadmap_status(
    text: &str,
    id: &NodeId,
    status: RoadmapStatus,
    evidence: Option<&Locator>,
) -> Result<String, AxError> {
    if matches!(status, RoadmapStatus::Done) && evidence.is_none() {
        return Err(AxError::failure(
            AxCode::EvidenceMissing,
            "mark a roadmap row done",
            format!("row {id} has no evidence"),
        )
        .with_recovery("pass a retrievable locator: `cas:<hash>` or `file:<path>@<oid>`"));
    }
    let rows = well_formed(text, "edit a roadmap row")?;
    let row = rows.iter().find(|row| &row.id == id).ok_or_else(|| {
        AxError::failure(
            AxCode::InvalidArgs,
            "edit a roadmap row",
            format!("no row numbered {id}"),
        )
        .with_recovery("read the roadmap and use an index the table carries")
    })?;
    let replacement = draw_row(row, status, evidence);
    Ok(rewrite(text, |cells| {
        cells
            .first()
            .and_then(|raw| NodeId::parse(raw).ok())
            .is_some_and(|found| &found == id)
            .then(|| replacement.clone())
    }))
}

/// Hangs new children under a node, returning the whole text.
///
/// The children land directly below the node's last descendant, so the
/// table stays in reading order and the numbers a person saw yesterday
/// still point at the same work. New ordinals continue after the
/// node's existing children rather than reusing a gap: a plan index is
/// a name, and reusing one would silently move somebody's evidence.
///
/// # Errors
/// Refuses a table that does not parse, an index no row carries, an
/// empty list of children, a child whose text is blank, a weight of
/// zero (a node worth nothing is a row nobody will ever be given), and
/// a split that would push the plan past `plan::NODE_DEPTH_MAX`.
pub fn insert_children(
    text: &str,
    parent: &NodeId,
    children: &[NewChild],
) -> Result<String, AxError> {
    let refuse = |subject: String, recovery: &str| {
        AxError::failure(AxCode::InvalidArgs, "split a plan node", subject)
            .with_recovery(recovery.to_owned())
    };
    if children.is_empty() {
        return Err(refuse(
            "no children given".to_owned(),
            "name at least one child; splitting into nothing would delete the work",
        ));
    }
    let rows = well_formed(text, "split a plan node")?;
    if !rows.iter().any(|row| &row.id == parent) {
        return Err(refuse(
            format!("no row numbered {parent}"),
            "read the roadmap and use an index the table carries",
        ));
    }
    let mut next = rows
        .iter()
        .filter(|row| row.id.parent().as_ref() == Some(parent))
        .map(|row| row.id.ordinal())
        .max()
        .unwrap_or(0);
    let mut drawn = Vec::with_capacity(children.len());
    for child in children {
        if child.item.trim().is_empty() {
            return Err(refuse(
                "a child with no text".to_owned(),
                "say what each child is; a row nobody can read is a row nobody can take",
            ));
        }
        if child.item.contains('|') {
            return Err(refuse(
                format!("`{}` carries a column separator", child.item),
                "write the item without `|`; it would split the row into more cells",
            ));
        }
        if child.weight == 0 {
            return Err(refuse(
                format!("`{}` is weighted zero", child.item),
                "give every child a weight of at least one; a node worth nothing is never given \
                 to anybody",
            ));
        }
        next = next.saturating_add(1);
        let id = parent.child(next)?;
        drawn.push(format!(
            "| {id} | {} | {} |  | {} |  |",
            child.item.trim(),
            child.weight,
            RoadmapStatus::NotStarted.spelling()
        ));
    }
    let Some((first, last)) = locate_table(text) else {
        return Err(refuse(
            "the roadmap table is not there".to_owned(),
            "repair the table before splitting a node",
        ));
    };
    // The insertion point: after the parent's last descendant, or after
    // the parent itself when it has none.
    let mut after = None;
    for (n, line) in text.split('\n').enumerate() {
        if n < first || n > last {
            continue;
        }
        let Some(cells) = split_table_row(line) else {
            continue;
        };
        if !body_row(&cells) {
            continue;
        }
        let Some(id) = cells.first().and_then(|raw| NodeId::parse(raw).ok()) else {
            continue;
        };
        if &id == parent || parent.is_ancestor_of(&id) {
            after = Some(n);
        }
    }
    let seam = after.unwrap_or(last);
    let mut out: Vec<String> = Vec::new();
    for (n, line) in text.split('\n').enumerate() {
        out.push(line.to_owned());
        if n == seam {
            out.extend(drawn.iter().cloned());
        }
    }
    Ok(out.join("\n"))
}

/// The rows of a well-formed table, or the refusal that says which line
/// is wrong.
fn well_formed(text: &str, action: &'static str) -> Result<Vec<RoadmapRow>, AxError> {
    match check_roadmap_shape(text) {
        RoadmapShape::WellFormed { rows } => Ok(rows),
        RoadmapShape::Malformed { problems } => Err(AxError::failure(
            AxCode::InvalidArgs,
            action,
            problems.join("; "),
        )
        .with_recovery(format!(
            "repair the {ROADMAP_COLUMNS}-column table in Roadmap.md first; a plan that does not \
             parse has no denominator"
        ))),
    }
}

/// Replaces the body rows of the first table for which `chosen` returns
/// a replacement, leaving every other byte alone.
fn rewrite(text: &str, mut chosen: impl FnMut(&[String]) -> Option<String>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_table = false;
    let mut done = false;
    let mut first = true;
    for line in text.split('\n') {
        if !first {
            out.push('\n');
        }
        first = false;
        let cells = split_table_row(line);
        if cells.is_some() {
            in_table = true;
        } else if in_table {
            in_table = false;
            done = true; // the first table ended; later tables are not ours
        }
        let hit = if done {
            None
        } else {
            cells
                .as_ref()
                .filter(|cells| body_row(cells))
                .and_then(|cells| chosen(cells))
        };
        match hit {
            Some(replacement) => out.push_str(&replacement),
            None => out.push_str(line),
        }
    }
    out
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests;
