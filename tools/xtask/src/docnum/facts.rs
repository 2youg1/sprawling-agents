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

use std::path::Path;

use crate::budget::REGISTER;
use crate::report::XtaskError;
use crate::{architecture, budget, depmap, gates, proof, walk};

mod crate_table;
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
const FACTS: [Fact; 21] = [
    Fact {
        key: "wire_v",
        home: "wire::WIRE_V",
        takes: None,
        recount: |_root, _arg| Ok(wire::WIRE_V.to_string()),
    },
    Fact {
        key: "command_frames",
        home: "wire::COMMAND_NAMES",
        takes: None,
        recount: |_root, _arg| Ok(wire::COMMAND_NAMES.len().to_string()),
    },
    Fact {
        key: "query_frames",
        home: "wire::QUERY_NAMES",
        takes: None,
        recount: |_root, _arg| Ok(wire::QUERY_NAMES.len().to_string()),
    },
    Fact {
        key: "command_names",
        home: "the `Command` variants of wire::wire_schema()",
        takes: None,
        recount: |_root, _arg| recount::wire_tags("Command"),
    },
    Fact {
        key: "query_names",
        home: "the `Query` variants of wire::wire_schema()",
        takes: None,
        recount: |_root, _arg| recount::wire_tags("Query"),
    },
    Fact {
        key: "gate_count",
        home: "the array in tools/xtask/src/gates.rs",
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
        home: "tools/fuzz/fuzz_targets",
        takes: None,
        recount: |root, _arg| recount::in_directory(root, "tools/fuzz/fuzz_targets", "fuzz target"),
    },
    Fact {
        key: "citysim_scenarios",
        home: "tools/citysim/tests",
        takes: None,
        recount: |root, _arg| recount::in_directory(root, "tools/citysim/tests", "scenario file"),
    },
    Fact {
        key: "test_functions",
        home: "the #[test] attributes in the tree",
        takes: None,
        recount: |root, _arg| recount::test_functions(root),
    },
    Fact {
        key: "crate_graph",
        home: "the depmap block in ARCHITECTURE.md section 3",
        takes: None,
        recount: |root, _arg| depmap::graph(&walk::read_text(&root.join(architecture::PATH))?),
    },
    Fact {
        key: "crate_table",
        home: "cargo metadata, the depmap block in ARCHITECTURE.md section 3, and the family \
               duties in architecture.toml",
        takes: None,
        recount: |root, _arg| crate_table::recount(root),
    },
    Fact {
        key: "adversary_seed",
        home: "tools/adversary/test/Main.lean",
        takes: None,
        recount: |root, _arg| adversary_seed(root),
    },
    Fact {
        key: "workspace_version",
        home: "the root Cargo.toml, [workspace.package] version",
        takes: None,
        recount: |root, _arg| workspace_version(root),
    },
    Fact {
        key: "dep_version",
        home: "the version this workspace pins in its manifests",
        takes: Some("the crate"),
        recount: recount::dep_version,
    },
    Fact {
        key: "budget_bytes",
        home: "the `budget_bytes` of that row in tools/xtask/budgets.toml",
        takes: Some("the register row"),
        recount: |root, arg| recount::grouped_bytes(root, arg, "budget_bytes"),
    },
    Fact {
        key: "budget_reading",
        home: "the `best_bytes` of that row in tools/xtask/budgets.toml",
        takes: Some("the register row"),
        recount: |root, arg| recount::grouped_bytes(root, arg, "best_bytes"),
    },
    Fact {
        key: "budget_headroom",
        home: "the budget over the reading, both from tools/xtask/budgets.toml",
        takes: Some("the register row"),
        recount: recount::headroom,
    },
    Fact {
        key: "budget_figure",
        home: "the named integer field of that row in tools/xtask/budgets.toml",
        takes: Some("the register row and its field, as `row.field`"),
        recount: recount::figure,
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

/// The root manifest, which states the workspace's package roster and
/// the versions every member inherits.
pub(crate) fn root_manifest(root: &Path) -> Result<toml::Value, XtaskError> {
    let text = walk::read_text(&root.join("Cargo.toml"))?;
    toml::from_str(&text).map_err(|err| XtaskError::Doc {
        file: "Cargo.toml".to_owned(),
        msg: format!("the root manifest does not parse: {err}"),
    })
}

/// The version every member inherits, which the newest `CHANGELOG.md`
/// section names in its heading.
fn workspace_version(root: &Path) -> Result<String, XtaskError> {
    root_manifest(root)?
        .get("workspace")
        .and_then(|workspace| workspace.get("package"))
        .and_then(|package| package.get("version"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| XtaskError::Doc {
            file: "Cargo.toml".to_owned(),
            msg: "no `[workspace.package] version`, so the release a document names cannot be                   recounted"
                .to_owned(),
        })
}

/// The register row a fact names, parsed from `tools/xtask/budgets.toml`.
fn register_row(root: &Path, row: &str) -> Result<toml::Value, XtaskError> {
    budget::register(root)?
        .get(row)
        .cloned()
        .ok_or_else(|| XtaskError::Doc {
            file: REGISTER.to_owned(),
            msg: format!("no `[{row}]` row, so nothing can be quoted from it"),
        })
}

/// The seed the adversary draws every property from, read out of the Lean
/// definition that holds it.
///
/// The figure's home is `tools/adversary/`, because that is the program that
/// draws from it, and `adversary-SPEC.md` quotes it. Reading it here makes
/// the quotation a managed span: nothing else joins a Lean definition to a
/// number written in prose, so the two could disagree with every gate
/// green.
///
/// A source that no longer states the seed in the one shape this reader
/// knows is a failure rather than an empty reading. Reporting nothing
/// would leave the quoted figure as the only copy, which is the shape of
/// defect the managed span exists to close.
fn adversary_seed(root: &Path) -> Result<String, XtaskError> {
    const SOURCE: &str = "tools/adversary/test/Main.lean";
    let text = walk::read_text(&root.join(SOURCE))?;
    let declared = text.lines().find_map(|line| {
        let value = line
            .trim_start()
            .strip_prefix("def defaultSeed")?
            .split_once(":=")
            .map(|(_, value)| value.trim())?;
        Some(value)
    });
    match declared {
        Some(value) if !value.is_empty() && value.bytes().all(|digit| digit.is_ascii_digit()) => {
            Ok(value.to_owned())
        }
        _ => Err(XtaskError::Doc {
            file: SOURCE.to_owned(),
            msg: "no `def defaultSeed ... := <digits>` line, so a document quoting the seed \
                  cannot be recounted against it"
                .to_owned(),
        }),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn root() -> std::path::PathBuf {
        crate::root::this_checkout().to_path_buf()
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
    fn a_register_figure_is_quoted_from_its_row_and_field() {
        let root = root();
        assert_eq!(
            value(&root, "budget_figure:views_rebuild_per_mb.best_p50_ms").unwrap(),
            Some("2,759".into())
        );
        assert!(value(&root, "budget_figure:views_rebuild_per_mb").is_err());
    }

    /// The relocated checkout's module map, with the two families its
    /// packages carry: `sprawling-j`'s modules are named `bin::`, as the
    /// assembly root's are, so it reads `[family.bin]` and not `[family.j]`.
    const FAMILIES: &str = r#"module = [
  { name = "k::a", file = "tools/k/src/a.rs", owns = "a module under tools/", shape = "value", since = "T", status = "built", spec = "k-SPEC.md#8-9" },
  { name = "bin::b", file = "crates/j/src/b.rs", owns = "a module named for its family", shape = "value", since = "T", status = "built", spec = "j-SPEC.md#8-1" },
]

[family.k]
duty = "what k owns | and a bar"

[family.bin]
duty = "the assembly root"
"#;

    /// A relocated checkout with a SPEC beside each package and `map` as
    /// its module map.
    fn tabled(label: &str, map: &str) -> std::path::PathBuf {
        let root = crate::root::fixture::relocated(label);
        crate::root::fixture::write(&root, "architecture.toml", map);
        crate::root::fixture::write(&root, "tools/k/k-SPEC.md", "");
        crate::root::fixture::write(&root, "crates/j/j-SPEC.md", "");
        root
    }

    #[test]
    fn the_crate_table_is_drawn_from_members_the_depmap_block_and_the_families() {
        let root = tabled("crate-table", FAMILIES);
        let drawn = value(&root, "crate_table");
        std::fs::remove_dir_all(&root).unwrap();
        let expected = r"| Directory | Package | Lib | Owns | May depend on | SPEC |
|---|---|---|---|---|---|
| `crates/j` | `sprawling-j` | `j` | the assembly root | `k` | `crates/j/j-SPEC.md` |
| `tools/k` | `sprawling-k` | `k` | what k owns \| and a bar | nothing | `tools/k/k-SPEC.md` |";
        assert_eq!(drawn.unwrap(), Some(expected.to_owned()));
    }

    #[test]
    fn a_crate_table_row_whose_family_is_missing_is_refused_by_name() {
        let without_k = FAMILIES.replace("[family.k]\nduty = \"what k owns | and a bar\"\n", "");
        let root = tabled("crate-table-no-family", &without_k);
        let drawn = value(&root, "crate_table");
        std::fs::remove_dir_all(&root).unwrap();
        assert!(
            matches!(&drawn, Err(XtaskError::Doc { msg, .. }) if msg.contains("[family.k]")),
            "{drawn:?}"
        );
    }

    #[test]
    fn the_workspace_version_is_read_from_the_root_manifest() {
        let fixture = std::env::temp_dir().join(format!("docnum-version-{}", std::process::id()));
        std::fs::create_dir_all(&fixture).unwrap();
        std::fs::write(
            fixture.join("Cargo.toml"),
            "[workspace]
members = []

[workspace.package]
version = \"9.8.7\"
",
        )
        .unwrap();
        let read = value(&fixture, "workspace_version");
        std::fs::remove_dir_all(&fixture).unwrap();
        assert_eq!(read.unwrap(), Some("9.8.7".into()));
    }
}
