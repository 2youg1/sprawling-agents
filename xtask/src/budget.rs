// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The performance ratchet: a number may improve freely, and may drift
//! back only within a stated slack.
//!
//! The register is `xtask/budgets.toml`, which holds every budget the
//! design states rather than only the ones a machine can check. A row it
//! cannot check says what it needs; an entry that quietly vanished
//! because nobody could measure it is how a budget stops existing.
//!
//! Only what a machine measures the same way twice is gated here. Sizes
//! qualify. Wall-clock figures do not: gating them would turn a busy
//! runner into a defect report, and the register says so per row.

use std::path::Path;

use crate::package::{ReleaseTarget, binary_path};
use crate::report::{Violation, XtaskError};

mod carried;
mod weighing;

use carried::CLIENT_MARK;
pub(crate) use carried::{carries_client, carries_engine};
pub(crate) use weighing::{lockfile_packages, measure};

/// What a gated row counts, and therefore which keys state its budget
/// and how a finding spells a reading.
///
/// Two units, because the register holds two kinds of fact a machine
/// measures the same way twice: how large a built artifact is, and how
/// many of something a resolved manifest names. A wall-clock figure is
/// never a unit here — the register says, per row, why the speed of the
/// machine that took a reading is recorded and not gated.
#[derive(Clone, Copy)]
enum Unit {
    Bytes,
    Packages,
}

impl Unit {
    /// The three keys a gated row of this unit states its budget, its
    /// best reading and its slack under.
    ///
    /// The suffix is the unit's name, so a row says what it counts
    /// without a second field repeating it, and a row that states bytes
    /// cannot be read as a row that states packages.
    fn keys(self) -> [&'static str; 3] {
        match self {
            Unit::Bytes => ["budget_bytes", "best_bytes", "slack_bytes"],
            Unit::Packages => ["budget_packages", "best_packages", "slack_packages"],
        }
    }

    /// One reading, in the words a finding uses.
    fn spell(self, reading: u64) -> String {
        match self {
            Unit::Bytes => format!("{reading} B"),
            Unit::Packages => format!("{reading} packages"),
        }
    }
}

/// Every unit a gated row may be stated in, walked when the register is
/// read. A unit added to the enum and left out here would be a unit no
/// row can use, so the array is beside the enum it enumerates.
const UNITS: [Unit; 2] = [Unit::Bytes, Unit::Packages];

/// The release optimisation level the register's size reading was taken
/// with. One word, and the reason the check above exists at all.
const PROFILE: &str = "z";

/// The register, as the gate reads it.
struct Row {
    name: String,
    unit: Unit,
    budget: u64,
    best: u64,
    slack: u64,
}

/// The register, parsed. Shared with `badge`, which renders the same
/// readings: two parsers would be two answers to "how big is it".
pub(crate) fn register(root: &Path) -> Result<toml::Value, XtaskError> {
    let path = root.join("xtask").join("budgets.toml");
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

    let mut violations = crate::badge::check(root)?;
    violations.extend(release_profile(root)?);
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
        let reading = row.unit.spell(measured);
        if measured > row.budget {
            violations.push(Violation {
                gate: "budget",
                location: format!("{} is {reading}", row.name),
                rule: "a reading stays inside the budget the design states".to_owned(),
                violation: format!(
                    "{reading} exceeds the {} budget",
                    row.unit.spell(row.budget)
                ),
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
                location: format!("{} is {reading}", row.name),
                rule: "a number may improve freely and drift only within its slack".to_owned(),
                violation: format!(
                    "{reading} is more than {} worse than the best recorded {}",
                    row.unit.spell(row.slack),
                    row.unit.spell(row.best)
                ),
                alternative: "recover the reading, or record the new one in xtask/budgets.toml \
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
pub(crate) fn report(root: &Path) -> Result<String, XtaskError> {
    let parsed = register(root)?;
    let mut out = String::from(
        "budget                 reading      best      budget
",
    );
    for row in gated_rows(&parsed) {
        let reading = match measure(root, &row.name)? {
            Some(bytes) => bytes.to_string(),
            None => "not built".to_owned(),
        };
        out.push_str(&format!(
            "{:<22} {:>9}  {:>9}  {:>10}
",
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
        out.push_str(&format!(
            "{name}: {stated}{status}
"
        ));
    }
    Ok(out)
}

/// The rows a machine can weigh: those marked gated that state a
/// budget, a best reading and a slack in one of the units above.
/// The release profile is the one the register's reading was taken with.
///
/// **A size is a fact about the linker that produced it**, so a reading
/// taken at one `opt-level` describes a binary nobody ships the moment
/// the profile says another. This is the rule the register already
/// states for its badge - one platform, and only that platform may
/// refresh the number - applied to the other half of what a size depends
/// on. It is a check rather than a comment because the setting is one
/// word in a manifest and the reading it invalidates is seven megabytes,
/// and it costs a read: the binary itself is weighed only when somebody
/// has built one.
///
/// `"z"` rather than `3` was decided by a criterion written down before
/// the readings existed, so no number could talk anybody into anything.
/// The register carried the losing arm as a row of its own until the day
/// this setting changed; that row is gone, because once `"z"` is what
/// ships, a second row describing the same binary is the second home the
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
            "set `opt-level = \"{PROFILE}\"`, or re-weigh the binary and move the register's              reading in the same commit"
        ),
    }])
}

fn gated_rows(parsed: &toml::Value) -> Vec<Row> {
    let mut rows = Vec::new();
    let Some(table) = parsed.as_table() else {
        return rows;
    };
    for (name, value) in table {
        if value.get("status").and_then(toml::Value::as_str) != Some("gated") {
            continue;
        }
        let number = |key: &str| value.get(key).and_then(toml::Value::as_integer);
        for unit in UNITS {
            let [budget_key, best_key, slack_key] = unit.keys();
            let (Some(budget), Some(best), Some(slack)) =
                (number(budget_key), number(best_key), number(slack_key))
            else {
                continue;
            };
            rows.push(Row {
                name: name.clone(),
                unit,
                budget: budget.unsigned_abs(),
                best: best.unsigned_abs(),
                slack: slack.unsigned_abs(),
            });
        }
    }
    rows
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
            budget_ms = 5
            status = "gated"
            "#,
        )
        .unwrap();
        let rows = gated_rows(&register);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "gated_one");
        assert_eq!(rows[0].budget, 100);
    }

    /// A row counting something other than bytes is gated on the same
    /// three questions, and its findings are spelled in its own unit.
    #[test]
    fn a_row_counted_in_packages_is_weighed_and_reported_in_packages() {
        let register: toml::Value = toml::from_str(
            r#"
            [dependency_count]
            budget_packages = 420
            best_packages = 389
            slack_packages = 16
            status = "gated"
            "#,
        )
        .unwrap();
        let rows = gated_rows(&register);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].budget, 420);
        assert_eq!(rows[0].unit.spell(390), "390 packages");
    }

    #[test]
    fn the_shipped_register_names_every_budget_and_each_says_how_it_is_measured() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let text = std::fs::read_to_string(root.join("xtask").join("budgets.toml")).unwrap();
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
