// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Edges stay in ARCHITECTURE.md `depmap`; `pub trait` only at registered
//! seams; `directions` for intra-crate edges.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::architecture;
use crate::members::{self, Member, Role};
use crate::report::{Violation, XtaskError};
use crate::walk;

use architecture::PATH as ARCH;

mod directions;

const SEAM_SECTION: u32 = 4;

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let text = walk::read_text(&root.join(ARCH))?;
    let allowed = parse_block(&text)?;
    let seams = seam_files(&text)?;
    let found = members::members(root)?;
    let mut violations = Vec::new();

    check_edges(&found, &allowed, &mut violations);
    check_pub_traits(root, &found, &seams, &mut violations)?;
    directions::check(root, &text, &mut violations)?;
    Ok(violations)
}

/// Mermaid flowchart of the depmap block.
///
/// # Errors
/// When `text` holds no closed depmap block, as [`check`] refuses it.
pub(crate) fn graph(text: &str) -> Result<String, XtaskError> {
    let lines = parse_block(text)?.into_iter().flat_map(|(name, deps)| {
        if deps.is_empty() {
            vec![format!("    {name}")]
        } else {
            deps.into_iter()
                .map(|dep| format!("    {name} --> {dep}"))
                .collect()
        }
    });
    Ok(["```mermaid".to_owned(), "flowchart TD".to_owned()]
        .into_iter()
        .chain(lines)
        .chain(["```".to_owned()])
        .collect::<Vec<_>>()
        .join("\n"))
}

/// Parse the sole ```depmap block into `name: dep, dep` rows.
///
/// # Errors
/// When the document holds no closed block, or a line in it has no colon.
pub(crate) fn parse_block(text: &str) -> Result<BTreeMap<String, BTreeSet<String>>, XtaskError> {
    let mut map = BTreeMap::new();
    let mut inside = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "```depmap" {
            inside = true;
            continue;
        }
        if inside {
            if trimmed == "```" {
                return Ok(map);
            }
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (name, deps) = trimmed.split_once(':').ok_or_else(|| XtaskError::Doc {
                file: ARCH.to_owned(),
                msg: format!("depmap block line without a colon: {trimmed:?}"),
            })?;
            let set: BTreeSet<String> = deps
                .split(',')
                .map(str::trim)
                .filter(|d| !d.is_empty())
                .map(str::to_owned)
                .collect();
            map.insert(name.trim().to_owned(), set);
        }
    }
    Err(XtaskError::Doc {
        file: ARCH.to_owned(),
        msg: "no closed ```depmap fenced block found in docs/ARCHITECTURE.md".to_owned(),
    })
}

/// Seam-table paths from §4.
///
/// # Errors
/// When the seam section is missing from the document.
fn seam_files(text: &str) -> Result<BTreeSet<String>, XtaskError> {
    let declared = architecture::section(text, SEAM_SECTION).ok_or_else(|| XtaskError::Doc {
        file: ARCH.to_owned(),
        msg: format!("no `## {SEAM_SECTION}` section, which is where the seams are declared"),
    })?;
    Ok(seams_in(&declared))
}

fn seams_in(declared: &[&str]) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for line in declared {
        if !line.trim_start().starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if cells.len() != 6 {
            continue;
        }
        if let Some(path) = cells.get(2)
            && path.ends_with(".rs")
            && seam_path(path)
        {
            set.insert((*path).to_owned());
        }
    }
    set
}

/// A seam cell is a repository-relative Rust path under a product root.
fn seam_path(path: &str) -> bool {
    const ROOTS: [&str; 5] = ["Sprawling/", "Tools/", "Calyx/", "Mei/", "RefRain/"];
    ROOTS.iter().any(|root| path.starts_with(root))
}

fn check_edges(
    found: &[Member],
    allowed: &BTreeMap<String, BTreeSet<String>>,
    violations: &mut Vec<Violation>,
) {
    let names: BTreeMap<&str, &str> = found
        .iter()
        .map(|member| (member.package.as_str(), member.name()))
        .collect();
    for member in found.iter().filter(|member| member.in_product_graph()) {
        let name = member.name();
        let Some(allowed_deps) = allowed.get(name) else {
            violations.push(Violation {
                gate: "depmap",
                location: member.dir.clone(),
                rule: "every product crate is registered in the depmap block (section 2)"
                    .to_owned(),
                violation: format!("crate {name} missing from the depmap block"),
                alternative: "register the crate and only the edges its responsibility needs in the depmap block"
                    .to_owned(),
            });
            continue;
        };
        let deps = member
            .depends_on
            .iter()
            .filter_map(|package| names.get(package.as_str()));
        for dep in deps.filter(|dep| !allowed_deps.contains(**dep)) {
            violations.push(Violation {
                gate: "depmap",
                location: format!("{}/Cargo.toml", member.dir),
                rule: "crate edges are a subset of the documented topology \
                       (section 2, C5); dependencies point inward only"
                    .to_owned(),
                violation: format!("hidden edge {name} -> {dep}"),
                alternative: "remove the dependency, or record the allowed edge in the depmap block and move the responsibility it serves"
                    .to_owned(),
            });
        }
    }
}

fn check_pub_traits(
    root: &Path,
    found: &[Member],
    seams: &BTreeSet<String>,
    violations: &mut Vec<Violation>,
) -> Result<(), XtaskError> {
    for member in found.iter().filter(|member| member.role == Role::Product) {
        for file in walk::files_with_ext(&root.join(&member.dir), &["rs"])? {
            let rel = walk::rel(root, &file);
            let text = walk::read_text(&file)?;
            for (index, line) in text.lines().enumerate() {
                if line.trim_start().starts_with("pub trait ") && !seams.contains(&rel) {
                    violations.push(Violation {
                        gate: "depmap",
                        location: format!("{rel}:{}", index.saturating_add(1)),
                        rule: "pub trait only at registered seams (section 3; \
                               one adapter = hypothetical seam)"
                            .to_owned(),
                        violation: "pub trait outside the seam list".to_owned(),
                        alternative: "use pub(crate) trait, or register the seam only with a real second adapter and its SPEC contract"
                            .to_owned(),
                    });
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn block_parses_names_and_edges() {
        let text = "x\n```depmap\nkernel:\nruntime: kernel, memory\n```\ny\n";
        let map = parse_block(text).unwrap();
        assert!(map.get("kernel").unwrap().is_empty());
        assert_eq!(map.get("runtime").unwrap().len(), 2);
    }

    #[test]
    fn the_crate_graph_draws_every_allowed_edge_from_dependent_to_dependency() {
        let text = "x\n```depmap\nkernel:\nmemory: kernel\nruntime: kernel, memory\n```\n";
        let expected = [
            "```mermaid",
            "flowchart TD",
            "    kernel",
            "    memory --> kernel",
            "    runtime --> kernel",
            "    runtime --> memory",
            "```",
        ]
        .join("\n");
        assert_eq!(graph(text).unwrap(), expected);
    }

    #[test]
    fn missing_block_is_a_doc_error() {
        assert!(parse_block("no block here").is_err());
    }

    #[test]
    fn seam_rows_are_four_cell_rows_with_paths() {
        let text = format!(
            "## {SEAM_SECTION} Seams\n\
             | a | Sprawling/kernel/src/ledger.rs | b | c |\n\
             | kernel::gate | Sprawling/kernel/src/gate.rs | x | 8.2 | S2 | 未建 |\n"
        );
        let seams = seam_files(&text).unwrap();
        assert!(seams.contains("Sprawling/kernel/src/ledger.rs"));
        assert_eq!(seams.len(), 1);
    }

    #[test]
    fn a_table_outside_the_seam_section_declares_no_seam() {
        let text = format!(
            "## {SEAM_SECTION} Seams\n\
             | a | Sprawling/kernel/src/ledger.rs | b | c |\n\
             ## 9 Shapes\n\
             | a | Sprawling/runtime/src/turn.rs | b | c |\n"
        );
        let seams = seam_files(&text).unwrap();
        assert_eq!(seams.len(), 1, "{seams:?}");
    }

    #[test]
    fn a_document_with_no_seam_section_is_a_doc_error() {
        assert!(seam_files("## 2 Stack\n").is_err());
    }

    #[test]
    fn a_relocated_package_is_judged_by_its_lib_name() {
        let root = crate::root::fixture::relocated("depmap");
        let found = check(&root);
        std::fs::remove_dir_all(&root).unwrap();
        assert!(matches!(&found, Ok(v) if v.is_empty()), "{found:?}");
    }
}
