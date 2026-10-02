// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The performance ratchet: a number may improve freely, and may drift
//! back only within a stated slack.
//!
//! The register is `tools/xtask/budgets.toml`, which holds every budget the
//! design states rather than only the ones a machine can check. A row it
//! cannot check says what it needs; an entry that quietly vanished
//! because nobody could measure it is how a budget stops existing.
//!
//! A row is weighed only when it is marked gated and states a budget, a
//! best reading and a slack in bytes. No row is gated today: runtime
//! speed comes before size, so the bytes of a built artifact are read
//! and printed like the lockfile's package count, and refused by nothing
//! (tools/xtask/Spec.lean §8-25). Wall-clock figures are not gated,
//! because gating them would turn a busy runner into a defect report,
//! and the register says so per row.

use std::path::Path;

use crate::package::{ReleaseTarget, binary_path};
use crate::report::{Violation, XtaskError};

mod carried;
mod weighing;

use carried::CLIENT_MARK;
pub(crate) use carried::{carries_client, carries_engine};
pub(crate) use weighing::{lockfile_packages, measure};

/// The key a gated row states its budget under, in bytes.
const BUDGET_KEY: &str = "budget_bytes";
/// The key a gated row states its best recorded reading under, in bytes.
const BEST_KEY: &str = "best_bytes";
/// The key a gated row states how far a reading may drift back past its
/// best, in bytes.
const SLACK_KEY: &str = "slack_bytes";

/// The release optimisation level the register's size reading was taken
/// with. One word, and the reason the check above exists at all.
const PROFILE: &str = "3";

/// A gated row of the register, in bytes.
struct Row {
    name: String,
    budget: u64,
    best: u64,
    slack: u64,
}

/// Where the register lives, repo-relative: every gate that reads a row
/// opens it here, and every message about a row names it.
pub(crate) const REGISTER: &str = "tools/xtask/budgets.toml";

/// The register, parsed. Shared with every gate that reads a row of it:
/// two parsers would be two answers to "how big is it".
pub(crate) fn register(root: &Path) -> Result<toml::Value, XtaskError> {
    let path = root.join(REGISTER);
    let text = std::fs::read_to_string(&path).map_err(|source| XtaskError::Io {
        path: path.display().to_string(),
        source,
    })?;
    toml::from_str(&text).map_err(|err| XtaskError::Doc {
        file: path.display().to_string(),
        msg: format!("the performance register does not parse: {err}"),
    })
}

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let parsed = register(root)?;

    let mut violations = release_profile(root)?;
    // The single-binary promise, checked as bytes: a release binary that
    // exists must carry the client bundle's file table. The placeholder
    // build (no `just build-web` beforehand) lacks the assets entry, and
    // a binary like that must not look shippable.
    if let Some(binary) = binary_path(root, &ReleaseTarget::Host)
        && !carries_client(&binary)?
    {
        violations.push(Violation {
            gate: "budget",
            location: binary.display().to_string(),
            rule: "a release binary carries the web client inside itself".to_owned(),
            violation: format!(
                "the embedded file table has no {CLIENT_MARK}: this binary would serve the \
                 placeholder page"
            ),
            alternative: "run `just build-web`, then rebuild the release binary".to_owned(),
        });
    }
    for row in gated_rows(&parsed) {
        let Some(measured) = measure(root, &row.name)? else {
            // Nothing built to weigh. Silence rather than a violation:
            // `just check` does not build a release binary or a wasm
            // bundle, and a gate that demanded them would be a gate
            // people learn to run with less.
            continue;
        };
        if measured > row.budget {
            violations.push(Violation {
                gate: "budget",
                location: format!("{} is {measured} B", row.name),
                rule: "a reading stays inside the budget the design states".to_owned(),
                violation: format!("{measured} B exceeds the {} B budget", row.budget),
                alternative:
                    "bring the reading back, or change the budget in the design and say why in \
                     the same change-set"
                        .to_owned(),
            });
            continue;
        }
        let ceiling = row.best.saturating_add(row.slack);
        if row.best > 0 && measured > ceiling {
            violations.push(Violation {
                gate: "budget",
                location: format!("{} is {measured} B", row.name),
                rule: "a number may improve freely and drift only within its slack".to_owned(),
                violation: format!(
                    "{measured} B is more than {} B worse than the best recorded {} B",
                    row.slack, row.best
                ),
                alternative:
                    "recover the reading, or record the new one in tools/xtask/budgets.toml \
                              with the reason it moved"
                        .to_owned(),
            });
        }
    }
    Ok(violations)
}

/// Reports every row: what it is, what it costs today, and what it is
/// allowed to cost. A gate that only speaks when it is unhappy leaves a
/// person guessing whether it measured anything at all.
///
/// A row outside the gated table is printed with its reading when
/// `measure` knows how to take one, and with its status alone when it
/// does not: a number nobody could measure is not printed as zero.
pub(crate) fn report(root: &Path) -> Result<String, XtaskError> {
    let parsed = register(root)?;
    let mut out = String::from("budget                 reading      best      budget\n");
    for row in gated_rows(&parsed) {
        let reading = match measure(root, &row.name)? {
            Some(bytes) => bytes.to_string(),
            None => "not built".to_owned(),
        };
        out.push_str(&format!(
            "{:<22} {:>9}  {:>9}  {:>10}\n",
            row.name, reading, row.best, row.budget
        ));
    }
    let Some(table) = parsed.as_table() else {
        return Ok(out);
    };
    let weighed: std::collections::BTreeSet<String> = gated_rows(&parsed)
        .into_iter()
        .map(|row| row.name)
        .collect();
    for (name, value) in table {
        if weighed.contains(name) {
            continue; // already weighed in the table above
        }
        let status = value
            .get("status")
            .and_then(toml::Value::as_str)
            .unwrap_or("");
        // A budget stated in something other than bytes still belongs in
        // the report: one that only the gate can see is one nobody
        // remembers is there.
        let stated = value
            .get("budget_lines")
            .and_then(toml::Value::as_integer)
            .map_or_else(String::new, |lines| format!("{lines} lines, "));
        let reading =
            measure(root, name)?.map_or_else(String::new, |reading| format!("{reading} \u{2014} "));
        out.push_str(&format!("{name}: {reading}{stated}{status}\n"));
    }
    Ok(out)
}

/// The release profile is the one the register's reading was taken with.
///
/// **A size is a fact about the linker that produced it**, so a reading
/// taken at one `opt-level` describes a binary nobody ships the moment
/// the profile says another. The register names the platform its
/// reading was linked on; this check holds the other half of what a
/// size depends on. It is a check rather than a comment because the
/// setting is one word in a manifest and the reading it invalidates is
/// the size of the whole binary, and it costs a read: the binary itself
/// is weighed only when somebody has built one.
///
/// `3` rather than `"z"` or `"s"` was decided on runtime speed over the
/// product's common operations, by a criterion written down before the
/// readings existed (the root `Cargo.toml` states it beside the setting).
/// The register carries no row for a losing arm, because once `3` is what
/// ships, a second row describing another binary is the second home the
/// register exists to prevent.
fn release_profile(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let path = root.join("Cargo.toml");
    let text = std::fs::read_to_string(&path).map_err(|source| XtaskError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let parsed: toml::Value = toml::from_str(&text).map_err(|err| XtaskError::Doc {
        file: path.display().to_string(),
        msg: err.to_string(),
    })?;
    let stated = parsed
        .get("profile")
        .and_then(|profile| profile.get("release"))
        .and_then(|release| release.get("opt-level"));
    // Both spellings a level can take, read the same way: `"z"` is a
    // string and `3` is an integer, and neither is more correct as TOML.
    let said = match stated {
        Some(toml::Value::String(text)) => text.clone(),
        Some(other) => other.to_string(),
        None => "unstated, which cargo reads as 3".to_owned(),
    };
    if said == PROFILE {
        return Ok(Vec::new());
    }
    Ok(vec![Violation {
        gate: "budget",
        location: "Cargo.toml [profile.release] opt-level".to_owned(),
        rule: "the release profile is the one the size reading was taken with".to_owned(),
        violation: format!(
            "`opt-level` is `{said}`, so the linked binary is not the one the register weighed"
        ),
        alternative: format!(
            "set `opt-level = \"{PROFILE}\"`, or re-weigh the binary and move the register's \
             reading in the same commit"
        ),
    }])
}

/// The rows a machine can weigh: those marked gated that state a
/// budget, a best reading and a slack in bytes.
fn gated_rows(parsed: &toml::Value) -> Vec<Row> {
    let Some(table) = parsed.as_table() else {
        return Vec::new();
    };
    table
        .iter()
        .filter(|(_, value)| value.get("status").and_then(toml::Value::as_str) == Some("gated"))
        .filter_map(|(name, value)| {
            let number = |key: &str| value.get(key).and_then(toml::Value::as_integer);
            Some(Row {
                name: name.clone(),
                budget: number(BUDGET_KEY)?.unsigned_abs(),
                best: number(BEST_KEY)?.unsigned_abs(),
                slack: number(SLACK_KEY)?.unsigned_abs(),
            })
        })
        .collect()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    #[test]
    fn only_rows_marked_gated_are_weighed() {
        let register: toml::Value = toml::from_str(
            r#"
            [gated_one]
            budget_bytes = 100
            best_bytes = 50
            slack_bytes = 10
            status = "gated"

            [measured_only]
            budget_bytes = 100
            best_bytes = 50
            slack_bytes = 10
            status = "measured, not gated: the counter differs per platform"

            [no_unit_this_gate_knows]
            budget_us = 5000
            status = "gated"
            "#,
        )
        .unwrap();
        let rows = gated_rows(&register);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "gated_one");
        assert_eq!(rows[0].budget, 100);
    }

    /// The package count is printed beside its row with its reading,
    /// and no count, however large, is a finding (tools/xtask/Spec.lean
    /// §8-25).
    #[test]
    fn a_package_count_is_reported_beside_its_row_and_refused_by_nothing() {
        let root = std::env::temp_dir().join(format!("xtask-budget-count-{}", std::process::id()));
        if root.exists() {
            std::fs::remove_dir_all(&root).unwrap();
        }
        let write = |path: &str, text: &str| crate::root::fixture::write(&root, path, text);
        write("Cargo.toml", "[profile.release]\nopt-level = 3\n");
        write(
            REGISTER,
            "[dependency_count]\n\
             what = \"packages in Cargo.lock once every transitive dependency is resolved, workspace members included\"\n\
             measured_by = \"cargo xtask budget (budget::lockfile_packages)\"\n\
             status = \"reported, not gated: the person's ruling\"\n",
        );
        write(
            "Cargo.lock",
            "version = 4\n\n[[package]]\nname = \"a\"\n\n[[package]]\nname = \"b\"\n\n[[package]]\nname = \"c\"\n",
        );

        let refused: Vec<String> = check(&root)
            .unwrap()
            .into_iter()
            .map(|violation| violation.location)
            .filter(|location| location.starts_with("dependency_count"))
            .collect();
        let printed = report(&root).unwrap();
        std::fs::remove_dir_all(&root).unwrap();

        assert_eq!(refused, Vec::<String>::new());
        assert!(
            printed.contains("dependency_count: 3 \u{2014} reported, not gated"),
            "the report does not print the reading beside its row:\n{printed}"
        );
    }

    #[test]
    fn the_shipped_register_names_every_budget_and_each_says_how_it_is_measured() {
        let text = std::fs::read_to_string(crate::root::this_checkout().join(REGISTER)).unwrap();
        let register: toml::Value = toml::from_str(&text).unwrap();
        let table = register.as_table().unwrap();
        // Not a count written twice: the register is the only place
        // that says how many budgets there are, and this asserts the
        // shape every row must have rather than how many rows exist.
        assert!(!table.is_empty(), "the register names no budget at all");
        for (name, row) in table {
            assert!(
                row.get("status").and_then(toml::Value::as_str).is_some(),
                "{name} does not say whether it is gated"
            );
            assert!(
                row.get("measured_by")
                    .and_then(toml::Value::as_str)
                    .is_some(),
                "{name} does not say how it is measured"
            );
        }
    }
}
