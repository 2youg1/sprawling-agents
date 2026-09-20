// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which facts a document may quote, and the code each one is recounted
//! from.
//!
//! [`FACTS`] is the whole authority. A row names the key a marker writes,
//! the home a refusal sends the reader to, and the function that recounts
//! it; adding a row is what lets a document quote one more number, and
//! there is no second list anywhere.
//!
//! A key may carry one argument after a colon — `dep_version:toml` names
//! the crate, `budget_reading:frontend_artifact` names the register row —
//! so one generator serves a family of facts instead of one row per
//! member. The argument is part of what the document writes, which is why
//! a wrong argument is refused by name rather than silently skipped.

use std::collections::BTreeSet;
use std::path::Path;

use crate::report::XtaskError;
use crate::{budget, gates, proof, walk};

mod recount;

/// One fact a document may quote, and how to recount it.
pub(super) struct Fact {
    /// The key a marker names it by, before any colon.
    pub(super) key: &'static str,
    /// Where the fact lives, for a refusal a reader can act on.
    pub(super) home: &'static str,
    /// What the argument after the colon names, when the fact takes one.
    pub(super) takes: Option<&'static str>,
    /// Today's value, rendered the way a document writes it. The second
    /// parameter is the argument, empty for a fact that takes none.
    recount: fn(&Path, &str) -> Result<String, XtaskError>,
}

/// Every fact a managed span may name.
const FACTS: [Fact; 16] = [
    Fact {
        key: "wire_v",
        home: "channels::WIRE_V",
        takes: None,
        recount: |_root, _arg| Ok(channels::WIRE_V.to_string()),
    },
    Fact {
        key: "command_frames",
        home: "channels::COMMAND_NAMES",
        takes: None,
        recount: |_root, _arg| Ok(channels::COMMAND_NAMES.len().to_string()),
    },
    Fact {
        key: "query_frames",
        home: "channels::QUERY_NAMES",
        takes: None,
        recount: |_root, _arg| Ok(channels::QUERY_NAMES.len().to_string()),
    },
    Fact {
        key: "command_names",
        home: "the `Command` variants of channels::wire_schema()",
        takes: None,
        recount: |_root, _arg| recount::wire_tags("Command"),
    },
    Fact {
        key: "query_names",
        home: "the `Query` variants of channels::wire_schema()",
        takes: None,
        recount: |_root, _arg| recount::wire_tags("Query"),
    },
    Fact {
        key: "gate_count",
        home: "the array in xtask/src/gates.rs",
        takes: None,
        recount: |_root, _arg| Ok(gates::COUNT.to_string()),
    },
    Fact {
        key: "dependency_count",
        home: "Cargo.lock",
        takes: None,
        recount: |root, _arg| recount::dependency_count(root),
    },
    Fact {
        key: "kani_harnesses",
        home: "the #[kani::proof] attributes in the tree",
        takes: None,
        recount: |root, _arg| Ok(proof::harnesses(root)?.len().to_string()),
    },
    Fact {
        key: "compile_fail_cases",
        home: "the trybuild cases under crates/*/tests/ui",
        takes: None,
        recount: |root, _arg| recount::files_under(root, "tests/ui", "trybuild case"),
    },
    Fact {
        key: "fuzz_targets",
        home: "fuzz/fuzz_targets",
        takes: None,
        recount: |root, _arg| recount::in_directory(root, "fuzz/fuzz_targets", "fuzz target"),
    },
    Fact {
        key: "citysim_scenarios",
        home: "citysim/tests",
        takes: None,
        recount: |root, _arg| recount::in_directory(root, "citysim/tests", "scenario file"),
    },
    Fact {
        key: "test_functions",
        home: "the #[test] attributes in the tree",
        takes: None,
        recount: |root, _arg| recount::test_functions(root),
    },
    Fact {
        key: "dep_version",
        home: "the version this workspace pins in its manifests",
        takes: Some("the crate"),
        recount: recount::dep_version,
    },
    Fact {
        key: "budget_bytes",
        home: "the `budget_bytes` of that row in xtask/budgets.toml",
        takes: Some("the register row"),
        recount: |root, arg| recount::grouped_bytes(root, arg, "budget_bytes"),
    },
    Fact {
        key: "budget_reading",
        home: "the `best_bytes` of that row in xtask/budgets.toml",
        takes: Some("the register row"),
        recount: |root, arg| recount::grouped_bytes(root, arg, "best_bytes"),
    },
    Fact {
        key: "budget_headroom",
        home: "the budget over the reading, both from xtask/budgets.toml",
        takes: Some("the register row"),
        recount: recount::headroom,
    },
];

/// Today's reading for one key, or `None` when no fact owns it.
///
/// A key that names a fact but carries the wrong shape of argument is
/// unknown rather than wrong: `budget_reading` without a row recounts
/// nothing, and the refusal that follows lists what a marker may say.
pub(super) fn value(root: &Path, key: &str) -> Result<Option<String>, XtaskError> {
    let (name, argument) = split(key);
    let Some(fact) = FACTS.iter().find(|fact| fact.key == name) else {
        return Ok(None);
    };
    match (fact.takes, argument) {
        (None, None) => (fact.recount)(root, "").map(Some),
        (Some(_), Some(argument)) if !argument.is_empty() => {
            (fact.recount)(root, argument).map(Some)
        }
        _ => Ok(None),
    }
}

/// Where the fact behind a key lives, for a refusal a reader can act on.
pub(super) fn home(key: &str) -> &'static str {
    let (name, _) = split(key);
    FACTS
        .iter()
        .find(|fact| fact.key == name)
        .map_or("its code", |fact| fact.home)
}

/// Every key a marker may name, in the shape it is written.
pub(super) fn keys() -> String {
    FACTS
        .iter()
        .map(|fact| match fact.takes {
            None => fact.key.to_owned(),
            Some(what) => format!("{}:<{what}>", fact.key),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// A key split into the fact it names and the argument it carries.
fn split(key: &str) -> (&str, Option<&str>) {
    match key.split_once(':') {
        Some((name, argument)) => (name.trim(), Some(argument.trim())),
        None => (key.trim(), None),
    }
}

/// Every package directory this workspace builds: its members, plus the
/// packages it excludes from the lint wall on purpose. Both lists live in
/// the root manifest, so nothing here holds a second roster.
///
/// The repository root is not among them. It holds the workspace table
/// rather than a package, and counting from it as well as from each
/// member would count every file in the tree twice.
pub(super) fn packages(root: &Path) -> Result<BTreeSet<String>, XtaskError> {
    let parsed = root_manifest(root)?;
    let mut out = BTreeSet::new();
    for list in ["members", "exclude"] {
        let Some(entries) = parsed
            .get("workspace")
            .and_then(|workspace| workspace.get(list))
            .and_then(toml::Value::as_array)
        else {
            continue;
        };
        for entry in entries {
            let Some(path) = entry.as_str() else {
                return Err(XtaskError::Doc {
                    file: "Cargo.toml".to_owned(),
                    msg: format!("`workspace.{list}` holds something that is not a path"),
                });
            };
            out.insert(path.to_owned());
        }
    }
    Ok(out)
}

/// The root manifest, which states the workspace's package roster and
/// the versions every member inherits.
pub(super) fn root_manifest(root: &Path) -> Result<toml::Value, XtaskError> {
    let text = walk::read_text(&root.join("Cargo.toml"))?;
    toml::from_str(&text).map_err(|err| XtaskError::Doc {
        file: "Cargo.toml".to_owned(),
        msg: format!("the root manifest does not parse: {err}"),
    })
}

/// The register row a fact names, parsed from `xtask/budgets.toml`.
fn register_row(root: &Path, row: &str) -> Result<toml::Value, XtaskError> {
    budget::register(root)?
        .get(row)
        .cloned()
        .ok_or_else(|| XtaskError::Doc {
            file: "xtask/budgets.toml".to_owned(),
            msg: format!("no `[{row}]` row, so nothing can be quoted from it"),
        })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;

    fn root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("the xtask manifest directory has a parent")
            .to_path_buf()
    }

    #[test]
    fn a_key_splits_into_its_fact_and_its_argument() {
        assert_eq!(split("wire_v"), ("wire_v", None));
        assert_eq!(split("dep_version:toml"), ("dep_version", Some("toml")));
        assert_eq!(split("dep_version:"), ("dep_version", Some("")));
    }

    #[test]
    fn an_argument_is_required_exactly_where_the_row_says_so() {
        let root = root();
        assert!(value(&root, "wire_v").unwrap().is_some());
        assert!(value(&root, "wire_v:toml").unwrap().is_none());
        assert!(value(&root, "dep_version").unwrap().is_none());
        assert!(value(&root, "dep_version:").unwrap().is_none());
        assert!(value(&root, "invented").unwrap().is_none());
    }

    #[test]
    fn every_fact_owns_its_key_and_recounts_on_this_tree() {
        let root = root();
        let named: BTreeSet<&str> = FACTS.iter().map(|fact| fact.key).collect();
        assert_eq!(named.len(), FACTS.len(), "two facts share one key");
        for fact in &FACTS {
            let key = match fact.takes {
                None => fact.key.to_owned(),
                Some(_) => continue,
            };
            let reading = value(&root, &key).expect("recounts").expect("is a fact");
            assert!(!reading.is_empty(), "{key} recounted to nothing");
        }
        assert_eq!(
            value(&root, "dep_version:toml").unwrap(),
            Some("1.1".into())
        );
        assert!(
            value(&root, "budget_reading:frontend_artifact")
                .unwrap()
                .is_some_and(|read| read.contains(','))
        );
    }

    #[test]
    fn the_package_roster_comes_from_the_root_manifest() {
        let found = packages(&root()).unwrap();
        assert!(found.contains("crates/kernel"));
        assert!(found.contains("desktop"), "an excluded package still pins");
        assert!(
            !found.contains(""),
            "the root is not a package; counting it would count the tree twice"
        );
    }
}
