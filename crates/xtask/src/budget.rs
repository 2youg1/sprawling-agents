// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Performance ratchet over `crates/xtask/budgets.toml` (xtask §8-25).

use std::path::Path;

use crate::package::{ReleaseTarget, binary_path};
use crate::report::{Violation, XtaskError};

mod carried;
mod weighing;

use carried::CLIENT_MARK;
pub(crate) use carried::{carries_client, carries_engine};
pub(crate) use weighing::{lockfile_packages, measure};

const BUDGET_KEY: &str = "budget_bytes";
const BEST_KEY: &str = "best_bytes";
const SLACK_KEY: &str = "slack_bytes";
const PROFILE: &str = "3";

struct Row {
    name: String,
    budget: u64,
    best: u64,
    slack: u64,
}

pub(crate) const REGISTER: &str = "crates/xtask/budgets.toml";

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
            continue;
        };
        if measured > row.budget {
            violations.push(Violation {
                gate: "budget",
                location: format!("{} is {measured} B", row.name),
                rule: "a reading stays inside the budget the design states".to_owned(),
                violation: format!("{measured} B exceeds the {} B budget", row.budget),
                alternative: "bring the reading back; if this budget no longer predicts a real defect, redefine its scope or delete it with evidence"
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
                alternative: "recover the reading; if the regression is intentional, record the measurement and the precise reason the old baseline stopped applying"
                    .to_owned(),
            });
        }
    }
    Ok(violations)
}

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
