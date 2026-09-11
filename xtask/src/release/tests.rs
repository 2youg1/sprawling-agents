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
        "README.zh-CN.md",
        "AGENTS.md",
        "ARCHITECTURE.md",
        "CLAUDE.md",
        "LICENSE",
        "docs/glossary.md",
        "docs/third-party.md",
        "docs/templates/JOB.md",
        "crates/kernel/kernel-SPEC.md",
        "crates/kernel/src/lib.rs",
        "xtask/lexicon.toml",
        "fixtures/golden.jsonl",
    ] {
        assert!(!is_scaffolding(out), "{out} is published");
    }
    // `localise.rs` is not the isolation zone, and a prefix rule
    // written without care would say it is.
    assert!(!is_scaffolding("crates/city/src/building.rs"));
}

#[test]
fn anything_the_rule_does_not_name_is_published() {
    // The failure direction is deliberate. An unclassified file
    // shows up in the artefact, where somebody sees it; the other
    // way round it disappears, where nobody does.
    assert!(!is_scaffolding("docs/some-new-page.md"));
    assert!(!is_scaffolding("crates/kernel/src/brand_new.rs"));
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
fn a_path_from_one_machine_is_caught_and_a_url_is_not() {
    // The false positive this shape invites is every URL in the
    // repository: `https://` ends in a letter, a colon and a slash.
    assert!(machine_path("see https://example.invalid/a/b").is_none());
    assert!(machine_path("run it in /tmp/city or /etc/hosts").is_none());
    assert!(machine_path("~/cities/first is fine").is_none());
    assert!(machine_path("nothing here").is_none());
    // A test that proves an absolute path is refused has to write
    // one, and `C:/windows` names a kind of machine rather than a
    // person. Flagging those would make the gate wrong about the
    // three files that assert the refusal.
    assert!(machine_path(r"C:\repo\crates").is_none());
    assert!(machine_path("C:/windows/system32").is_none());

    assert!(machine_path(r"C:\Users\someone\WORKSPACE\city").is_some());
    assert!(machine_path("d:/users/someone/work").is_some());
    assert!(machine_path("/home/someone/cities/first").is_some());
    assert!(machine_path("/Users/someone/cities").is_some());
    assert!(machine_path("stored under /root/city").is_some());
    // The report carries enough to find it and not the whole line.
    let found = machine_path(r"the log said C:\Users\someone\a\b\c\d\e").unwrap();
    assert!(found.chars().count() <= 20);
    assert!(!found.contains("the log said"));
    // A line of Chinese prose puts the byte offset well past the
    // character count; the report must still carry the path.
    let chinese = machine_path(r"日志里写的是 C:\Users\someone\city").unwrap();
    assert!(chinese.starts_with(":\\users\\"), "{chinese}");
}

#[test]
fn a_document_anchored_outside_the_repository_is_caught_and_shorthand_is_not() {
    let dirs: BTreeSet<String> = ["crates", "docs", "src", "tools", "xtask", "local", "target"]
        .into_iter()
        .map(str::to_owned)
        .collect();

    // The case that happened. Six files cited this; no other
    // assertion in this gate said anything about it.
    assert_eq!(
        outside_the_tree("see `WORKSPACE/FRONTEND-METHOD.md` section 4", &dirs).as_deref(),
        Some("WORKSPACE/FRONTEND-METHOD.md")
    );
    assert!(outside_the_tree("WORKSPACE/sprawling-ui/UX-DECISIONS.md", &dirs).is_some());

    // Crate-relative shorthand is how every SPEC here is written.
    // Flagging it would send eleven people to rewrite prose that was
    // never wrong.
    assert!(outside_the_tree("// tools/exec.rs", &dirs).is_none());
    assert!(outside_the_tree("`read src/lex.rs` differs by", &dirs).is_none());
    assert!(outside_the_tree("crates/city/src/building.rs", &dirs).is_none());
    assert!(outside_the_tree("see local/Handoff.md", &dirs).is_none());

    // A URL is somebody else's tree. The colon ends the token, so
    // what reaches the test has an empty first segment.
    assert!(outside_the_tree("https://example.invalid/a/b.md", &dirs).is_none());
    // A relative walk upwards is the link assertion's business, and
    // a path inside a city is not a path inside this repository.
    assert!(outside_the_tree("[a](../../docs/frontend-method.md)", &dirs).is_none());
    assert!(outside_the_tree("edit `.sprawling/BUILDING.md`", &dirs).is_none());
    // The build directory is not published and still exists.
    assert!(outside_the_tree("written to target/screens/tokens.css", &dirs).is_none());
    // A word with no extension is not a citation.
    assert!(outside_the_tree("and/or, either/or", &dirs).is_none());
    // Trailing punctuation must not hide the extension.
    assert!(outside_the_tree("open WORKSPACE/notes.md.", &dirs).is_some());
}

#[test]
fn only_prose_is_read_for_citations() {
    // A city address in a fixture is not a path in this repository,
    // and there are seventy-nine of them.
    assert!(!is_prose(
        "crates/city/src/building.rs",
        "    assert!(a(\"lab/x.md\"));"
    ));
    assert!(is_prose(
        "crates/city/src/building.rs",
        "//! see docs/glossary.md"
    ));
    assert!(is_prose("crates/city/city-SPEC.md", "anything at all"));
    assert!(is_prose("docs/templates/BUILDING.md", "<!-- a comment -->"));
}

#[test]
fn directory_names_carry_no_depth() {
    let names = directory_names(&[
        "crates/runtime/src/tools/exec.rs".to_owned(),
        "README.md".to_owned(),
    ]);
    for present in ["crates", "runtime", "src", "tools", "local"] {
        assert!(names.contains(present), "{present} is a directory here");
    }
    assert!(!names.contains("WORKSPACE"));
    // The file itself is not a directory.
    assert!(!names.contains("exec.rs"));
    assert!(!names.contains("README.md"));
}

#[test]
fn a_bracket_that_is_not_a_link_is_not_a_target() {
    assert!(link_targets("an array [1, 2] (three)").is_empty());
    assert!(link_targets("nothing here at all").is_empty());
}

#[test]
fn the_repository_itself_passes_the_check_it_ships() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .expect("xtask lives one level under the repo root");
    let violations = check(&root).expect("the check runs");
    assert!(
        violations.is_empty(),
        "product documents link to scaffolding:\n{}",
        violations
            .iter()
            .map(|v| format!("{}: {}", v.location, v.violation))
            .collect::<Vec<String>>()
            .join("\n")
    );
}
