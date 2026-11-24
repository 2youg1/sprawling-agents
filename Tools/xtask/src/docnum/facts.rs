// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use crate::budget::REGISTER;
use crate::report::XtaskError;
use crate::{architecture, budget, depmap, gates, proof, walk};

mod crate_table;
mod recount;

pub(super) struct Fact {
    pub(super) key: &'static str,
    pub(super) home: &'static str,
    pub(super) takes: Option<&'static str>,
    recount: fn(&Path, &str) -> Result<String, XtaskError>,
}

const FACTS: &[Fact] = &[
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
        home: "the array in Tools/xtask/src/gates.rs",
        takes: None,
        recount: |_root, _arg| Ok(gates::COUNT.to_string()),
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
        home: "Tools/fuzz/fuzz_targets",
        takes: None,
        recount: |root, _arg| recount::in_directory(root, "Tools/fuzz/fuzz_targets", "fuzz target"),
    },
    Fact {
        key: "citysim_scenarios",
        home: "Tools/citysim/tests/scenarios/main.rs",
        takes: None,
        recount: |root, _arg| recount::citysim_scenarios(root),
    },
    Fact {
        key: "crate_graph",
        home: "the depmap block in docs/ARCHITECTURE.md section 3",
        takes: None,
        recount: |root, _arg| depmap::graph(&walk::read_text(&root.join(architecture::PATH))?),
    },
    Fact {
        key: "crate_table",
        home: "cargo metadata, the depmap block in docs/ARCHITECTURE.md section 3, and the family \
               duties in architecture.toml",
        takes: None,
        recount: |root, _arg| crate_table::recount(root),
    },
    Fact {
        key: "adversary_seed",
        home: "Tools/adversary/test/Main.lean",
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
        home: "the `budget_bytes` of that row in Tools/xtask/budgets.toml",
        takes: Some("the register row"),
        recount: |root, arg| recount::grouped_bytes(root, arg, "budget_bytes"),
    },
    Fact {
        key: "budget_reading",
        home: "the `best_bytes` of that row in Tools/xtask/budgets.toml",
        takes: Some("the register row"),
        recount: |root, arg| recount::grouped_bytes(root, arg, "best_bytes"),
    },
    Fact {
        key: "budget_headroom",
        home: "the budget over the reading, both from Tools/xtask/budgets.toml",
        takes: Some("the register row"),
        recount: recount::headroom,
    },
    Fact {
        key: "budget_figure",
        home: "the named integer field of that row in Tools/xtask/budgets.toml",
        takes: Some("the register row and its field, as `row.field`"),
        recount: recount::figure,
    },
];

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

pub(super) fn home(key: &str) -> &'static str {
    let (name, _) = split(key);
    FACTS
        .iter()
        .find(|fact| fact.key == name)
        .map_or("its code", |fact| fact.home)
}

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

fn split(key: &str) -> (&str, Option<&str>) {
    match key.split_once(':') {
        Some((name, argument)) => (name.trim(), Some(argument.trim())),
        None => (key.trim(), None),
    }
}

pub(crate) fn root_manifest(root: &Path) -> Result<toml::Value, XtaskError> {
    let text = walk::read_text(&root.join("Cargo.toml"))?;
    toml::from_str(&text).map_err(|err| XtaskError::Doc {
        file: "Cargo.toml".to_owned(),
        msg: format!("the root manifest does not parse: {err}"),
    })
}

fn workspace_version(root: &Path) -> Result<String, XtaskError> {
    root_manifest(root)?
        .get("workspace")
        .and_then(|workspace| workspace.get("package"))
        .and_then(|package| package.get("version"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| XtaskError::Doc {
            file: "Cargo.toml".to_owned(),
            msg: "no `[workspace.package] version`, so the release a document names cannot be \
            recounted"
                .to_owned(),
        })
}

fn register_row(root: &Path, row: &str) -> Result<toml::Value, XtaskError> {
    budget::register(root)?
        .get(row)
        .cloned()
        .ok_or_else(|| XtaskError::Doc {
            file: REGISTER.to_owned(),
            msg: format!("no `[{row}]` row, so nothing can be quoted from it"),
        })
}

fn adversary_seed(root: &Path) -> Result<String, XtaskError> {
    const SOURCE: &str = "Tools/adversary/test/Main.lean";
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
        for fact in FACTS {
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
        let root = crate::root::fixture::relocated("register-figure");
        crate::root::fixture::write(
            &root,
            "Tools/xtask/budgets.toml",
            "[views_rebuild_per_mb]
best_p50_us = 2759
",
        );
        assert_eq!(
            value(&root, "budget_figure:views_rebuild_per_mb.best_p50_us").unwrap(),
            Some("2,759".into())
        );
        assert!(value(&root, "budget_figure:views_rebuild_per_mb").is_err());
    }

    const FAMILIES: &str = r#"module = [
  { name = "k::a", file = "tools/k/src/a.rs", owns = "a module under tools/", shape = "value", since = "T", status = "built", spec = "k-SPEC.md#8-9" },
  { name = "bin::b", file = "crates/j/src/b.rs", owns = "a module named for its family", shape = "value", since = "T", status = "built", spec = "j-SPEC.md#8-1" },
]

[family.k]
duty = "what k owns | and a bar"

[family.bin]
duty = "the assembly root"
"#;

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
    fn a_migrated_package_quotes_its_lean_specification() {
        let root = crate::root::fixture::relocated("crate-table-lean");
        crate::root::fixture::write(&root, "architecture.toml", FAMILIES);
        crate::root::fixture::write(&root, "tools/k/Spec.lean", "");
        crate::root::fixture::write(&root, "crates/j/j-SPEC.md", "");
        let drawn = value(&root, "crate_table");
        std::fs::remove_dir_all(&root).unwrap();
        let cells = drawn.map_err(|err| err.to_string()).map(|table| {
            table.map(|text| {
                text.lines()
                    .skip(2)
                    .filter_map(|row| row.trim_end_matches(" |").rsplit("| ").next())
                    .map(str::to_owned)
                    .collect::<Vec<String>>()
            })
        });
        assert_eq!(
            cells,
            Ok(Some(vec![
                "`crates/j/j-SPEC.md`".to_owned(),
                "`tools/k/Spec.lean`".to_owned()
            ]))
        );
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
