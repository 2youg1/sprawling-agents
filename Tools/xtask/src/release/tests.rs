// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

#[test]
fn one_machine_stays_behind_and_everything_that_explains_the_code_goes_out() {
    for behind in ["local", "local/Handoff.md", "local/shots/01.png"] {
        assert!(is_scaffolding(behind), "{behind} belongs to one machine");
    }
    for out in [
        "README.md",
        "docs/README.zh-CN.md",
        "AGENTS.md",
        "docs/ARCHITECTURE.md",
        "CLAUDE.md",
        "LICENSE",
        "docs/glossary.md",
        "docs/third-party.md",
        "Sprawling/city/templates/JOB.md",
        "Sprawling/kernel/kernel-SPEC.md",
        "Sprawling/kernel/src/lib.rs",
        "Tools/xtask/lexicon.toml",
        "Tools/fixtures/golden.jsonl",
    ] {
        assert!(!is_scaffolding(out), "{out} is published");
    }
    assert!(!is_scaffolding("Sprawling/city/src/building.rs"));
}

#[test]
fn anything_the_rule_does_not_name_is_published() {
    assert!(!is_scaffolding("docs/some-new-page.md"));
    assert!(!is_scaffolding("Sprawling/kernel/src/brand_new.rs"));
}

#[test]
fn links_are_read_as_written_and_resolved_against_their_file() {
    let text = "see [the plan](../local/Handoff.md) and [glossary](glossary.md#terms)";
    let targets = link_targets(text);
    assert_eq!(targets, ["../local/Handoff.md", "glossary.md#terms"]);
    assert_eq!(
        resolve("docs/CONTRIBUTING.md", "../local/Handoff.md"),
        "local/Handoff.md"
    );
    assert_eq!(
        resolve("docs/CONTRIBUTING.md", "glossary.md"),
        "docs/glossary.md"
    );
    assert_eq!(resolve("README.md", "docs/glossary.md"), "docs/glossary.md");
}

#[test]
fn a_link_that_differs_only_in_case_is_caught_here_and_nowhere_else() {
    let published: BTreeSet<String> = [
        "docs/glossary.md",
        "docs/getting-started.md",
        "AGENTS.md",
        "Sprawling/kernel/kernel-SPEC.md",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let spellings = Spellings::of(&published);

    assert_eq!(
        spellings.miscased("docs/Glossary.md"),
        Some("docs/glossary.md")
    );
    assert_eq!(
        spellings.miscased("DOCS/GETTING-STARTED.MD"),
        Some("docs/getting-started.md")
    );
    assert_eq!(
        spellings.miscased("Sprawling/kernel/kernel-spec.md"),
        Some("Sprawling/kernel/kernel-SPEC.md")
    );

    assert_eq!(spellings.miscased("docs/glossary.md"), None);
    assert_eq!(spellings.miscased("AGENTS.md"), None);
    assert_eq!(spellings.miscased("docs"), None);
    assert_eq!(spellings.miscased("local/Handoff.md"), None);
    assert_eq!(spellings.miscased("docs/never-written.md"), None);
}

#[test]
fn a_path_from_one_machine_is_caught_and_a_url_is_not() {
    assert!(machine_path("see https://example.invalid/a/b").is_none());
    assert!(machine_path("run it in /tmp/city or /etc/hosts").is_none());
    assert!(machine_path("~/cities/first is fine").is_none());
    assert!(machine_path("nothing here").is_none());
    assert!(machine_path(r"C:\repo\crates").is_none());
    assert!(machine_path("C:/windows/system32").is_none());

    assert!(machine_path(r"C:\Users\someone\WORKSPACE\city").is_some());
    assert!(machine_path("d:/users/someone/work").is_some());
    assert!(machine_path("/home/someone/cities/first").is_some());
    assert!(machine_path("/Users/someone/cities").is_some());
    assert!(machine_path("stored under /root/city").is_some());
    let found = machine_path(r"the log said C:\Users\someone\a\b\c\d\e").unwrap();
    assert!(found.chars().count() <= 20);
    assert!(!found.contains("the log said"));
    let chinese = machine_path(r"日志里写的是 C:\Users\someone\city").unwrap();
    assert!(chinese.starts_with(":\\users\\"), "{chinese}");
}

#[test]
fn a_document_anchored_outside_the_repository_is_caught_and_shorthand_is_not() {
    let dirs: BTreeSet<String> = [
        "Sprawling",
        "Tools",
        "docs",
        "src",
        "client",
        "local",
        "target",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();

    assert_eq!(
        outside_the_tree("see `WORKSPACE/FRONTEND-METHOD.md` section 4", &dirs).as_deref(),
        Some("WORKSPACE/FRONTEND-METHOD.md")
    );
    assert!(outside_the_tree("WORKSPACE/sprawling-ui/UX-DECISIONS.md", &dirs).is_some());

    assert!(outside_the_tree("// Tools/xtask/src/lex.rs", &dirs).is_none());
    assert!(outside_the_tree("`read src/lex.rs` differs by", &dirs).is_none());
    assert!(outside_the_tree("Sprawling/city/src/building.rs", &dirs).is_none());
    assert!(outside_the_tree("see local/Handoff.md", &dirs).is_none());

    assert!(outside_the_tree("https://example.invalid/a/b.md", &dirs).is_none());
    assert!(outside_the_tree("[a](../../docs/frontend-method.md)", &dirs).is_none());
    assert!(outside_the_tree("edit `.sprawling/RULES.toml`", &dirs).is_none());
    assert!(outside_the_tree("written to target/screens/tokens.css", &dirs).is_none());
    assert!(outside_the_tree("and/or, either/or", &dirs).is_none());
    assert!(outside_the_tree("open WORKSPACE/notes.md.", &dirs).is_some());
}

#[test]
fn only_prose_is_read_for_citations() {
    assert!(!is_prose(
        "Sprawling/city/src/building.rs",
        "    assert!(a(\"lab/x.md\"));"
    ));
    assert!(is_prose(
        "Sprawling/city/src/building.rs",
        "//! see docs/glossary.md"
    ));
    assert!(is_prose("Sprawling/city/city-SPEC.md", "anything at all"));
    assert!(is_prose(
        "Sprawling/city/templates/SPEC.md",
        "<!-- a comment -->"
    ));
}

#[test]
fn directory_names_carry_no_depth() {
    let names = directory_names(&[
        "Sprawling/runtime/src/tools/exec.rs".to_owned(),
        "README.md".to_owned(),
    ]);
    for present in ["Sprawling", "runtime", "src", "tools", "local"] {
        assert!(names.contains(present), "{present} is a directory here");
    }
    assert!(!names.contains("WORKSPACE"));
    assert!(!names.contains("exec.rs"));
    assert!(!names.contains("README.md"));
}

#[test]
fn a_bracket_that_is_not_a_link_is_not_a_target() {
    assert!(link_targets("an array [1, 2] (three)").is_empty());
    assert!(link_targets("nothing here at all").is_empty());
}

#[test]
fn a_lean_specification_is_prose_and_cites_lean() {
    let dirs: BTreeSet<String> = ["crates".to_owned()].into_iter().collect();
    assert_eq!(
        (
            is_prose("crates/x/Spec.lean", "theorem t : True := trivial"),
            outside_the_tree("see nowhere/Spec.lean", &dirs),
        ),
        (true, Some("nowhere/Spec.lean".to_owned()))
    );
}
