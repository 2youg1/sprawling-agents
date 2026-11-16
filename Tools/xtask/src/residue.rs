// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Gate: retired legacy paths leave no residue in active configuration
//! (xtask D45). Historical CHANGELOG files are outside the scan by
//! filename scope. A legacy path whose status is still `live` does not
//! fail the present layout.

use std::path::Path;

use crate::modmap;
use crate::product_roots::{self, Layout, LegacyPath, Table};
use crate::release;
use crate::report::{Violation, XtaskError};
use crate::walk;

const GATE: &str = "residue";
const RULE: &str =
    "a retired legacy path must not remain in the tree or in active configuration (xtask D45)";

/// Filename stems treated as historical record, never as active config.
const HISTORY_FILES: [&str; 1] = ["CHANGELOG.md"];

/// Extensions of active configuration the residue scan reads.
const ACTIVE_EXT: [&str; 8] = ["toml", "yml", "yaml", "nix", "json", "sh", "ps1", "just"];

/// Gate entry: validate the product-root table, refuse a wrong-case
/// sibling of a current root, and refuse retired legacy residue.
/// Shell implementation paths that must stay deleted under `Calyx/src`
/// (`Calyx/Spec.lean` D7). Checked as files or directories.
const CALYX_SHELL_RESIDUE: &[&str] = &[
    "Calyx/src/console/ui",
    "Calyx/src/console/ui.rs",
    "Calyx/src/console/editor.rs",
    "Calyx/src/console/screen.rs",
    "Calyx/src/console/program_status",
    "Calyx/src/console/program_status.rs",
];

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let table = product_roots::load(root)?;
    let mut violations = Vec::new();
    violations.extend(missing_current(root, &table));
    violations.extend(wrong_case(root, &table)?);
    for legacy in table.retired() {
        violations.extend(retired_residue(root, legacy, &table)?);
    }
    violations.extend(calyx_shell_residue(root));
    Ok(violations)
}

fn calyx_shell_residue(root: &Path) -> Vec<Violation> {
    let rule = "Calyx shell implementation stays in Zig; Rust under Calyx/src/console/{ui,editor,screen,program_status} is permanent residue (Calyx Spec D7)";
    let mut violations = Vec::new();
    for rel in CALYX_SHELL_RESIDUE {
        let path = root.join(rel);
        if !path.exists() {
            continue;
        }
        let kind = if path.is_dir() { "directory" } else { "file" };
        violations.push(Violation {
            gate: GATE,
            location: (*rel).to_owned(),
            rule: rule.to_owned(),
            violation: format!("Calyx shell residue `{rel}` still exists as a {kind}"),
            alternative: format!(
                "delete `{rel}`; the interactive shell lives in `Calyx/shell` and the city host in `Calyx/src/console/host.rs`"
            ),
        });
    }
    violations
}

fn missing_current(root: &Path, table: &Table) -> Vec<Violation> {
    let mut violations = Vec::new();
    for product in table
        .roots
        .iter()
        .filter(|product| product.layout == Layout::Current)
    {
        let Some(path) = table.path_of(&product.name) else {
            continue;
        };
        if root.join(path).is_dir() {
            continue;
        }
        violations.push(Violation {
            gate: GATE,
            location: format!("{path}/"),
            rule: RULE.to_owned(),
            violation: format!(
                "product root `{name}` declares path `{path}` with layout `current`, and that directory is not there",
                name = product.name
            ),
            alternative: format!(
                "create `{path}/`, or set `layout = \"planned\"` in `{map}` until the move lands",
                map = modmap::MAP
            ),
        });
    }
    violations
}

fn wrong_case(root: &Path, table: &Table) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    for product in table
        .roots
        .iter()
        .filter(|root| root.layout == Layout::Current)
    {
        if let Some(found) = sibling_wrong_case(root, &product.path)? {
            violations.push(Violation {
                gate: GATE,
                location: found.clone(),
                rule: RULE.to_owned(),
                violation: format!(
                    "directory `{found}` is a wrong-case sibling of product root `{}`",
                    product.path
                ),
                alternative: format!(
                    "remove `{found}`, or rename it to the exact case `{}` from `{map}`",
                    product.path,
                    map = modmap::MAP
                ),
            });
        }
    }
    Ok(violations)
}

fn sibling_wrong_case(root: &Path, exact: &str) -> Result<Option<String>, XtaskError> {
    let entries = std::fs::read_dir(root).map_err(|source| XtaskError::Io {
        path: root.display().to_string(),
        source,
    })?;
    let want = exact.to_ascii_lowercase();
    for entry in entries {
        let entry = entry.map_err(|source| XtaskError::Io {
            path: root.display().to_string(),
            source,
        })?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !entry.path().is_dir() {
            continue;
        }
        if name != exact && name.to_ascii_lowercase() == want {
            return Ok(Some(name.to_owned()));
        }
    }
    Ok(None)
}

fn retired_residue(
    root: &Path,
    legacy: &LegacyPath,
    table: &Table,
) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    let path = root.join(&legacy.path);
    if path.exists() {
        let kind = if path.is_dir() { "directory" } else { "file" };
        violations.push(Violation {
            gate: GATE,
            location: legacy.path.clone(),
            rule: RULE.to_owned(),
            violation: format!(
                "retired legacy path `{}` still exists as a {kind}",
                legacy.path
            ),
            alternative: format!(
                "finish the move that retired `{}`, or set its status back to `live` in `{map}` while the tree still holds it",
                legacy.path,
                map = modmap::MAP
            ),
        });
    }
    for file in walk::files(root)? {
        let rel = walk::rel(root, &file);
        if !is_active_config(&rel) {
            continue;
        }
        let text = walk::read_text(&file)?;
        for (number, line) in text.lines().enumerate() {
            if defines_legacy_row(&rel, line, legacy) {
                continue;
            }
            if transition_row(&rel, line, table, legacy) {
                continue;
            }
            if !line_names_legacy(line, &legacy.path) {
                continue;
            }
            violations.push(Violation {
                gate: GATE,
                location: format!("{rel}:{}", number.saturating_add(1)),
                rule: RULE.to_owned(),
                violation: format!(
                    "active configuration still names retired path `{}`",
                    legacy.path
                ),
                alternative: format!(
                    "rewrite the path from `{map}`, or move historical prose into a CHANGELOG",
                    map = modmap::MAP
                ),
            });
        }
    }
    Ok(violations)
}

fn is_active_config(rel: &str) -> bool {
    let name = rel.rsplit('/').next().unwrap_or(rel);
    if HISTORY_FILES
        .iter()
        .any(|history| name.eq_ignore_ascii_case(history))
    {
        return false;
    }
    if name.eq_ignore_ascii_case("justfile") || name.eq_ignore_ascii_case("lakefile.toml") {
        return true;
    }
    if rel.starts_with(".github/") {
        return true;
    }
    Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ACTIVE_EXT.iter().any(|keep| keep.eq_ignore_ascii_case(ext)))
}

fn defines_legacy_row(rel: &str, line: &str, legacy: &LegacyPath) -> bool {
    rel == modmap::MAP && line.contains(&format!("path = \"{}\"", legacy.path))
}

fn transition_row(rel: &str, line: &str, table: &Table, legacy: &LegacyPath) -> bool {
    if rel != modmap::MAP {
        return false;
    }
    table.roots.iter().any(|root| {
        root.transition.as_deref() == Some(legacy.path.as_str())
            && line.contains(&format!("transition = \"{}\"", legacy.path))
    })
}

fn line_names_legacy(line: &str, legacy: &str) -> bool {
    let needle = format!("{legacy}/");
    let normalized = line.replace('\\', "/");
    release::path_tokens(&normalized).into_iter().any(|token| {
        let trimmed = token.trim_end_matches(['.', '-']);
        let rest = trimmed.strip_prefix("./").unwrap_or(trimmed);
        rest.starts_with(&needle)
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;
    use crate::root::fixture;
    use crate::root::this_checkout;

    fn minimal_table(retired_crates: bool) -> String {
        let status = if retired_crates { "retired" } else { "live" };
        format!(
            r#"
module = []
[[product_root]]
name = "Sprawling"
path = "Sprawling"
layout = "planned"
[[product_root]]
name = "Tools"
path = "Tools"
layout = "planned"
[[product_root]]
name = "Calyx"
path = "Calyx"
layout = "current"
[[product_root]]
name = "RefRain"
path = "RefRain"
layout = "current"
[[product_root]]
name = "Mei"
path = "Mei"
layout = "current"
[[legacy_path]]
path = "crates"
status = "{status}"
[[legacy_path]]
path = "client"
status = "retired"
"#
        )
    }

    fn scratch(label: &str) -> std::path::PathBuf {
        let root =
            std::env::temp_dir().join(format!("xtask-residue-{label}-{}", std::process::id()));
        if root.exists() {
            std::fs::remove_dir_all(&root).unwrap();
        }
        fixture::write(&root, "Cargo.lock", "");
        std::fs::create_dir_all(root.join("Calyx")).unwrap();
        std::fs::create_dir_all(root.join("Mei")).unwrap();
        std::fs::create_dir_all(root.join("RefRain")).unwrap();
        root
    }

    #[test]
    fn the_checkout_layout_is_green_after_refrain_r0() {
        let violations = check(this_checkout()).unwrap();
        assert!(
            violations.is_empty(),
            "residue must stay green on the post-F3 layout: {violations:?}"
        );
    }

    #[test]
    fn a_retired_path_in_active_config_is_refused() {
        let root = scratch("retired-ref");
        fixture::write(&root, modmap::MAP, &minimal_table(true));
        fixture::write(&root, "Cargo.toml", "members = [\"crates/kernel\"]\n");
        let violations = check(&root).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
        assert!(
            violations.iter().any(|v| v.violation.contains("crates")),
            "{violations:?}"
        );
    }

    #[test]
    fn changelog_files_are_outside_the_scan_by_filename_scope() {
        let root = scratch("history");
        fixture::write(&root, modmap::MAP, &minimal_table(true));
        fixture::write(
            &root,
            "docs/CHANGELOG.md",
            "- moved out of `crates/kernel`\n",
        );
        let violations = check(&root).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
        assert!(violations.is_empty(), "{violations:?}");
    }

    #[test]
    fn an_active_config_path_is_not_skippable_by_a_marker_comment() {
        let root = scratch("no-waiver");
        fixture::write(&root, modmap::MAP, &minimal_table(true));
        fixture::write(
            &root,
            "Cargo.toml",
            "# historical note\nmembers = [\"crates/kernel\"]\n",
        );
        let violations = check(&root).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
        assert!(
            violations.iter().any(|v| v.violation.contains("crates")),
            "{violations:?}"
        );
    }

    #[test]
    fn the_isolation_zone_is_outside_the_scan() {
        let root = scratch("local");
        fixture::write(&root, modmap::MAP, &minimal_table(true));
        fixture::write(&root, "local/notes.toml", "old = \"crates/kernel\"\n");
        let violations = check(&root).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
        assert!(violations.is_empty(), "{violations:?}");
    }

    #[test]
    fn a_quoted_label_that_is_not_a_path_is_not_residue() {
        let root = scratch("label");
        fixture::write(&root, modmap::MAP, &minimal_table(true));
        fixture::write(
            &root,
            ".github/workflows/ci.yml",
            "if: matrix.environment == 'crates'\n",
        );
        let violations = check(&root).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
        assert!(violations.is_empty(), "{violations:?}");
    }

    #[test]
    fn a_wrong_case_sibling_of_a_current_root_is_refused() {
        let root = scratch("case");
        fixture::write(&root, modmap::MAP, &minimal_table(false));
        std::fs::remove_dir(root.join("Calyx")).unwrap();
        std::fs::create_dir_all(root.join("calyx")).unwrap();
        let violations = check(&root).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
        assert!(
            violations.iter().any(|v| v.violation.contains("calyx")),
            "{violations:?}"
        );
    }

    #[test]
    fn line_names_legacy_matches_path_tokens_only() {
        assert!(line_names_legacy("members = [\"crates/kernel\"]", "crates"));
        assert!(!line_names_legacy("the crates themselves", "crates"));
        assert!(line_names_legacy("path = \"./crates/xtask\"", "crates"));
        assert!(line_names_legacy(
            "cargo --manifest-path crates\\kernel\\Cargo.toml",
            "crates"
        ));
        assert!(!line_names_legacy("members = [\"crates\"]", "crates"));
        assert!(!line_names_legacy(
            "matrix.environment == 'client'",
            "client"
        ));
    }

    #[test]
    fn a_windows_path_in_active_config_is_refused_when_retired() {
        let root = scratch("backslash");
        fixture::write(&root, modmap::MAP, &minimal_table(true));
        fixture::write(
            &root,
            "build.ps1",
            "cargo --manifest-path crates\\kernel\\Cargo.toml\n",
        );
        let violations = check(&root).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
        assert!(
            violations.iter().any(|v| v.violation.contains("crates")),
            "{violations:?}"
        );
    }

    #[test]
    fn calyx_shell_residue_refuses_a_restored_ui_module() {
        let root = scratch("calyx-shell");
        fixture::write(&root, modmap::MAP, &minimal_table(false));
        fixture::write(&root, "Calyx/src/console/ui.rs", "// restored\n");
        let violations = check(&root).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
        assert!(
            violations
                .iter()
                .any(|v| v.location.contains("Calyx/src/console/ui.rs")),
            "{violations:?}"
        );
    }

    #[test]
    fn calyx_shell_residue_refuses_a_restored_program_status_module() {
        let root = scratch("calyx-shell-ps");
        fixture::write(&root, modmap::MAP, &minimal_table(false));
        fixture::write(
            &root,
            "Calyx/src/console/program_status/mod.rs",
            "// restored\n",
        );
        let violations = check(&root).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
        assert!(
            violations
                .iter()
                .any(|v| v.location.contains("Calyx/src/console/program_status")),
            "{violations:?}"
        );
    }
}
