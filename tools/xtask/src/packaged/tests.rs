// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a published package may compile in: its own files and its build
//! script's output, judged along the module tree cargo would compile,
//! with test code and unpublished packages left alone.

use super::*;
use crate::root::fixture::write;

/// A workspace of three packages: `p`, published, with the package `q`
/// nested in its directory, and `t`, which says `publish = false`.
fn workspace(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("xtask-packaged-{label}-{}", std::process::id()));
    if root.exists() {
        std::fs::remove_dir_all(&root).unwrap();
    }
    for (path, text) in FILES {
        write(&root, path, text);
    }
    root
}

const FILES: [(&str, &str); 10] = [
    (
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/p\", \"crates/p/q\", \"tools/t\"]\nresolver = \"3\"\n",
    ),
    (
        "crates/p/Cargo.toml",
        "[package]\nname = \"p\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
    ),
    (
        "crates/p/src/lib.rs",
        "const OUTSIDE: &str = include_str!(\"../../x.txt\");\n\
         const INSIDE: &str = include_str!(\"../data.txt\");\n\
         const BUILT: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/made.bin\"));\n\
         const NESTED: &str = include_str!(\"../q/z.txt\").trim_ascii_end();\n\
         #[path = \"elsewhere/loaded.rs\"]\n\
         mod loaded;\n\
         mod plain;\n\
         #[cfg(test)]\n\
         mod tests;\n\
         #[cfg(all(test, windows))]\n\
         const ONLY_TESTED: &str = include_str!(\"../../../w.txt\");\n",
    ),
    (
        "crates/p/src/elsewhere/loaded.rs",
        "const FAR: &[u8] = include_bytes!(\"../../../y.bin\");\n",
    ),
    (
        "crates/p/src/plain.rs",
        "mod deeper;\nconst HERE: &str = include_str!(\"plain/deeper.rs\");\n",
    ),
    (
        "crates/p/src/plain/deeper.rs",
        "fn f() -> &'static str { format!(\"{}\", include_str!(\"../../../../v.txt\")).leak() }\n",
    ),
    (
        "crates/p/src/tests.rs",
        "const FIXTURE: &str = include_str!(\"../../../tools/fixtures/a.jsonl\");\n",
    ),
    (
        "crates/p/q/Cargo.toml",
        "[package]\nname = \"q\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
    ),
    ("crates/p/q/src/lib.rs", ""),
    (
        "tools/t/Cargo.toml",
        "[package]\nname = \"t\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n\n\
         [lib]\npath = \"src/lib.rs\"\n",
    ),
];

fn locations(found: &[Violation]) -> Vec<&str> {
    found.iter().map(|it| it.location.as_str()).collect()
}

/// The defect this gate exists for, in each place a production module
/// can hide it: the crate root, a file loaded through `#[path]`, a file
/// two `mod` declarations down, a call nested inside another macro, and
/// a path into a package nested in this one's directory, which cargo
/// leaves out of this one's archive. Test code, the build script's
/// output and the package's own files are not reported.
#[test]
fn every_file_a_published_package_reaches_outside_itself_is_named() {
    let root = workspace("reach");
    write(
        &root,
        "tools/t/src/lib.rs",
        "const OUTSIDE: &str = include_str!(\"../../x.txt\");\n",
    );
    let found = check(&root).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!(
        locations(&found),
        [
            "crates/p/src/elsewhere/loaded.rs:1",
            "crates/p/src/lib.rs:1",
            "crates/p/src/lib.rs:4",
            "crates/p/src/plain/deeper.rs:1",
        ],
        "{found:#?}"
    );
    assert!(found.iter().all(|it| it.gate == "packaged"), "{found:#?}");
    assert!(
        found[1].violation.contains("../../x.txt"),
        "{:#?}",
        found[1]
    );
}

/// An argument whose path this gate cannot read is a finding rather
/// than a pass: a guess here would be a green gate over a build that
/// fails on the first `cargo install`.
#[test]
fn an_argument_the_gate_cannot_read_is_named() {
    let root = workspace("unread");
    write(&root, "tools/t/src/lib.rs", "");
    write(
        &root,
        "crates/p/src/plain/deeper.rs",
        "const NAMED: &str = include_str!(concat!(env!(\"HOME\"), \"/x\"));\n",
    );
    let found = check(&root).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    let unread: Vec<&Violation> = found
        .iter()
        .filter(|it| it.location == "crates/p/src/plain/deeper.rs:1")
        .collect();
    assert_eq!(unread.len(), 1, "{found:#?}");
    assert!(unread[0].alternative.contains("OUT_DIR"), "{unread:#?}");
}

/// A `mod` declaration whose file is not there leaves the gate unable
/// to say what the package compiles, which is a refusal to judge.
#[test]
fn a_module_without_its_file_is_a_gate_that_cannot_judge() {
    let root = workspace("missing");
    write(&root, "tools/t/src/lib.rs", "");
    write(&root, "crates/p/src/plain.rs", "mod gone;\n");
    let verdict = check(&root);
    std::fs::remove_dir_all(&root).unwrap();
    assert!(
        matches!(&verdict, Err(XtaskError::Doc { msg, .. }) if msg.contains("gone")),
        "{verdict:?}"
    );
}

fn meta(text: &str) -> syn::Meta {
    syn::parse_str(text).unwrap()
}

/// Which `cfg` predicates keep an item out of every build that is not a
/// test build: the ones that cannot hold while `test` is false.
#[test]
fn a_predicate_excludes_production_only_when_it_cannot_hold_without_test() {
    let read: Vec<(&str, bool)> = [
        "test",
        "all(test, windows)",
        "any(test, all(test, unix))",
        "any(test, feature = \"conformance\")",
        "not(test)",
        "windows",
        "all()",
        "any()",
    ]
    .into_iter()
    .map(|text| (text, excludes_production(&meta(text))))
    .collect();
    assert_eq!(
        read,
        [
            ("test", true),
            ("all(test, windows)", true),
            ("any(test, all(test, unix))", true),
            ("any(test, feature = \"conformance\")", false),
            ("not(test)", false),
            ("windows", false),
            ("all()", false),
            ("any()", true),
        ]
    );
}
