// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

/// The skeleton lands in the directory cargo metadata gives the lib.
#[test]
fn a_relocated_package_gets_its_spec_skeleton_in_its_own_directory() {
    let root = crate::root::fixture::relocated("spec");
    let made = super::run(&root, Some("k"));
    let landed = root.join("tools/k/Spec.lean").is_file();
    std::fs::remove_dir_all(&root).unwrap();
    assert!(made.is_ok() && landed, "{made:?}");
}

/// The skeleton is Lean: the notice as Lean comments, then the seventeen
/// section comments `crates/city/skills/sdd` lists, numbered in order.
#[test]
fn the_skeleton_opens_with_the_notice_and_numbers_the_seventeen_sections_in_order() {
    let text = super::skeleton("k");
    let head: Vec<String> = text.lines().take(4).map(str::to_owned).collect();
    let numbers: Vec<String> = text
        .lines()
        .filter_map(|line| line.strip_prefix("/-! ## "))
        .map(|rest| rest.split(' ').next().unwrap().to_owned())
        .collect();
    assert_eq!(
        (head, numbers),
        (
            crate::header::notice(crate::header::Leader::Lean).to_vec(),
            (1..=17).map(|n: u8| n.to_string()).collect::<Vec<_>>()
        )
    );
}

/// Everything the gate reports on a fixture, as `location: violation`,
/// sorted; the fixture is removed before anything is asserted.
fn judged(root: &std::path::Path) -> Result<Vec<String>, String> {
    for (path, text) in [
        ("architecture.toml", "module = []\n"),
        (
            "tools/xtask/budgets.toml",
            "[spec_parts_without_theorem]\npinned = 0\n\n[rust_paths_unresolved]\npinned = 0\n",
        ),
    ] {
        if !root.join(path).exists() {
            crate::root::fixture::write(root, path, text);
        }
    }
    let found = super::check(root);
    std::fs::remove_dir_all(root).unwrap();
    let mut texts: Vec<String> = found
        .map_err(|err| err.to_string())?
        .into_iter()
        .map(|v| format!("{}: {}", v.location, v.violation))
        .collect();
    texts.sort();
    Ok(texts)
}

/// Two specifications in one package, and none in another.
#[test]
fn a_package_holds_exactly_one_effective_specification() {
    let root = crate::root::fixture::relocated("spec-effective");
    crate::root::fixture::write(&root, "tools/k/k-SPEC.md", "# k\n");
    crate::root::fixture::write(&root, "tools/k/Spec.lean", "");
    assert_eq!(
        judged(&root),
        Ok(vec![
            "crates/j: crates/j holds neither a `*-SPEC.md` nor a Spec.lean".to_owned(),
            "tools/k: tools/k holds tools/k/k-SPEC.md and tools/k/Spec.lean".to_owned(),
        ])
    );
}

/// A SPEC named in prose is in the tree; a pattern, a string in code and
/// the history are not citations.
#[test]
fn prose_names_no_spec_the_tree_lacks() {
    let root = crate::root::fixture::relocated("spec-dangling");
    crate::root::fixture::write(&root, "tools/k/k-SPEC.md", "# k\n");
    crate::root::fixture::write(&root, "crates/j/j-SPEC.md", "# j\n");
    crate::root::fixture::write(
        &root,
        "docs/a.md",
        "Read k-SPEC §3 and gone-SPEC.md; <lib>-SPEC.md is a pattern, SPECIFIC-SPECS a word.\n",
    );
    crate::root::fixture::write(
        &root,
        "crates/j/src/lib.rs",
        "// see x-SPEC\nconst S: &str = \"y-SPEC.md\";\n",
    );
    crate::root::fixture::write(&root, "CHANGELOG.md", "old-SPEC.md\n");
    assert_eq!(
        judged(&root),
        Ok(vec![
            "crates/j/src/lib.rs:1: names `x-SPEC`, and no `x-SPEC.md` is in the tree".to_owned(),
            "docs/a.md:1: names `gone-SPEC`, and no `gone-SPEC.md` is in the tree".to_owned(),
        ])
    );
}

/// The imports each side may make, no proof left undischarged, and the
/// paths a specification cites.
#[test]
fn lean_imports_along_the_crate_graph_and_proves_what_it_states() {
    let root = crate::root::fixture::relocated("spec-source");
    crate::root::fixture::write(&root, "tools/k/k-SPEC.md", "# k\n");
    crate::root::fixture::write(
        &root,
        "crates/j/Spec.lean",
        "import tools.k.Spec\nimport Sprawling.Frame\n\
         /-! Specifies crates/j/src/lib.rs and crates/j/src/gone.rs; no sorry here. -/\n\
         theorem t : True := by sorry\nprivate axiom bad : False\n\
         structure S where\n  admit : Nat\ndef S.go (s : S) := s.admit\n\
         theorem u : True := by\n  admit\n",
    );
    crate::root::fixture::write(&root, "tools/k/spec/A.lean", "import crates.j.Spec\n");
    crate::root::fixture::write(
        &root,
        "tools/adversary/src/X.lean",
        "import Lean.Data.Json\nimport crates.j.Spec\n",
    );
    assert_eq!(
        judged(&root),
        Ok(vec![
            "crates/j/Spec.lean:10: leaves an `admit`".to_owned(),
            "crates/j/Spec.lean:2: imports `Sprawling.Frame`, which is neither the toolchain's \
             nor a crate's specification"
                .to_owned(),
            "crates/j/Spec.lean:3: cites `crates/j/src/gone.rs`, which is not there".to_owned(),
            "crates/j/Spec.lean:4: leaves a `sorry`".to_owned(),
            "crates/j/Spec.lean:5: declares an axiom".to_owned(),
            "tools/adversary/src/X.lean:2: the checker imports `crates.j.Spec`, and it imports \
             only `Sprawling` and the toolchain's libraries"
                .to_owned(),
            "tools/k/spec/A.lean:1: imports `crates.j.Spec`, a part of j, which the depmap block \
             does not let k depend on"
                .to_owned(),
        ])
    );
}

/// The checker's own specification: its entry imports its parts, a part
/// may not import the checker, the checker may not import it, the paths
/// it cites are on disk, and it is the checker's one specification.
#[test]
fn the_checker_has_one_specification_that_imports_its_parts_and_never_the_checker() {
    let root = crate::root::fixture::relocated("spec-checker");
    crate::root::fixture::write(&root, "tools/k/k-SPEC.md", "# k\n");
    crate::root::fixture::write(&root, "tools/adversary/adversary-SPEC.md", "# adversary\n");
    crate::root::fixture::write(
        &root,
        "tools/adversary/Spec.lean",
        "import tools.adversary.spec.Answer\n/-! Specifies crates/j/src/gone.rs. -/\n",
    );
    crate::root::fixture::write(
        &root,
        "tools/adversary/spec/Answer.lean",
        "import Lean.Data.Json\nimport Sprawling.Door\nimport crates.j.Spec\n",
    );
    crate::root::fixture::write(
        &root,
        "tools/adversary/src/Sprawling/X.lean",
        "import Sprawling.Frame\nimport tools.adversary.Spec\n",
    );
    assert_eq!(
        judged(&root),
        Ok(vec![
            "crates/j: crates/j holds neither a `*-SPEC.md` nor a Spec.lean".to_owned(),
            "tools/adversary/Spec.lean:2: cites `crates/j/src/gone.rs`, which is not there"
                .to_owned(),
            "tools/adversary/spec/Answer.lean:2: the checker's specification imports \
             `Sprawling.Door`, and it imports only its own parts and the toolchain's libraries"
                .to_owned(),
            "tools/adversary/spec/Answer.lean:3: the checker's specification imports \
             `crates.j.Spec`, and it imports only its own parts and the toolchain's libraries"
                .to_owned(),
            "tools/adversary/src/Sprawling/X.lean:2: the checker imports `tools.adversary.Spec`, \
             and it imports only `Sprawling` and the toolchain's libraries"
                .to_owned(),
            "tools/adversary: tools/adversary holds tools/adversary/adversary-SPEC.md and \
             tools/adversary/Spec.lean"
                .to_owned(),
        ])
    );
}

/// The four classes, read off the text: a theorem in a comment is prose,
/// a head with no binder is closed, and a check is derived only from a
/// Rust file that holds a test and names the part and the theorem.
#[test]
fn a_part_is_text_closed_quantified_or_checked() {
    use super::theorems::Class;
    let files = |pairs: &[(&str, &str)]| -> std::collections::BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(path, text)| ((*path).to_owned(), (*text).to_owned()))
            .collect()
    };
    let tree = super::ratchet::Tree {
        lean: files(&[
            (
                "a/spec/T.lean",
                "-- theorem fake : x\n/-! theorem prose : y -/\n",
            ),
            ("a/spec/C.lean", "theorem two : 1 + 1 = 2 := rfl\n"),
            (
                "a/spec/Q.lean",
                "theorem keeps (n : Nat) : n = n := rfl\n\
                 theorem all : ∀ n : Nat, n = n := fun _ => rfl\n",
            ),
            (
                "a/spec/D.lean",
                "@[simp] private theorem grows {s : S}\n    (h : s.ok) : P s := by\n  simp\n",
            ),
            ("tools/adversary/src/X.lean", ""),
            ("tools/adversary/spec/Y.lean", ""),
        ]),
        rust: files(&[
            ("a/tests.rs", "#[test]\nfn f() {} // a/spec/D.lean grows\n"),
            ("a/doc.rs", "// a/spec/Q.lean keeps\n"),
        ]),
        markdown: files(&[]),
    };
    let classes: Vec<(String, Class)> = super::ratchet::classes(&tree).into_iter().collect();
    assert_eq!(
        classes,
        vec![
            ("a/spec/C.lean".to_owned(), Class::ClosedOnly),
            ("a/spec/D.lean".to_owned(), Class::QuantifiedChecked),
            ("a/spec/Q.lean".to_owned(), Class::QuantifiedUnchecked),
            ("a/spec/T.lean".to_owned(), Class::TextOnly),
            ("tools/adversary/spec/Y.lean".to_owned(), Class::TextOnly),
        ]
    );
}

/// Both ratchets on one fixture: the stateful module's two parts prove
/// nothing and exceed their pin, so each is named; three backticked paths
/// name nothing, below a pin of five, so the pin is asked to fall. A
/// re-export, a crate-root item, a variant, a module named with a note,
/// a path outside the workspace and the history resolve or are not read.
#[test]
fn the_two_counts_are_held_to_their_pins() {
    let root = crate::root::fixture::relocated("spec-ratchet");
    let write = |path: &str, text: &str| crate::root::fixture::write(&root, path, text);
    write("tools/k/k-SPEC.md", "# k\n");
    write("crates/j/Spec.lean", "");
    write(
        "architecture.toml",
        "module = [\n\
         { name = \"j::machine (port)\", file = \"crates/j/src/machine.rs\", owns = \"m\", \
         shape = \"state machine\", status = \"built\", spec = \"crates.j.spec.Machine\" },\n\
         { name = \"j::plain\", file = \"crates/j/src/plain.rs\", owns = \"p\", \
         shape = \"decision\", status = \"planned\", spec = \"crates.j.spec.Plain\" },\n\
         ]\n",
    );
    write(
        "tools/xtask/budgets.toml",
        "[spec_parts_without_theorem]\npinned = 1\n\n[rust_paths_unresolved]\npinned = 5\n",
    );
    write("crates/j/src/lib.rs", "pub use machine::Phase;\n");
    write(
        "crates/j/src/machine.rs",
        "pub(crate) enum Phase {\n    Idle,\n}\npub(crate) use other::{Thing, More};\n\
         /// see `j::plain::Gone`\n",
    );
    write(
        "crates/j/spec/Machine.lean",
        "/-! `j::machine::Phase::Idle`, `j::gone::X()` and `j::machine::Missing` -/\n",
    );
    write("crates/j/spec/Machine/Step.lean", "/-! steps -/\n");
    write("crates/j/spec/Plain.lean", "/-! plain -/\n");
    write(
        "docs/a.md",
        "`j::machine::Thing`, `j::Phase::Idle`, `std::mem::swap` and `j::machine`\n",
    );
    write("CHANGELOG.md", "`j::gone`\n");
    assert_eq!(
        judged(&root),
        Ok(vec![
            "crates/j/spec/Machine.lean: [spec_parts_without_theorem] counts 2 against a pin of \
             1; this is one of them"
                .to_owned(),
            "crates/j/spec/Machine/Step.lean: [spec_parts_without_theorem] counts 2 against a \
             pin of 1; this is one of them"
                .to_owned(),
            "tools/xtask/budgets.toml: [rust_paths_unresolved]: [rust_paths_unresolved] counts \
             3, below its pin of 5"
                .to_owned(),
        ])
    );
}
