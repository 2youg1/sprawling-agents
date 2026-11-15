// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(clippy::unwrap_used, reason = "test code")]

use super::*;
use crate::header::{self, Leader};

#[test]
fn the_boundary_is_fifteen_percent_and_one_more_comment_is_rejected() {
    let mut source = header::notice(Leader::Rust).join("\n");
    source.push('\n');
    source.push_str(&"// necessary contract\n".repeat(15));
    source.push_str(&"const VALUE: u8 = 1;\n".repeat(85));
    let accepted = lexer::measure(&source, Language::Rust);
    assert_eq!(
        accepted,
        Reading {
            comment_lines: 15,
            judged_lines: 100,
        }
    );
    assert!(!over(accepted));
    source.push_str("// one too many\n");
    assert!(over(lexer::measure(&source, Language::Rust)));
}

#[test]
fn an_inline_comment_counts_and_an_empty_file_is_clean() {
    let reading = lexer::measure(
        "const FIRST: u8 = 1; // why\nconst SECOND: u8 = 2;\n",
        Language::TypeScript,
    );
    assert_eq!(reading.comment_lines, 1);
    assert_eq!(reading.judged_lines, 2);
    assert!(over(reading));
    assert!(!over(lexer::measure("", Language::Rust)));
}

#[test]
fn rust_literals_and_lifetimes_do_not_hide_real_comments() {
    let source = r####"let raw = r###"// not a comment
/* also not a comment */
"###;
let text = "/* not a comment */";
let character = '/';
fn lifetimes<'a, 'b>() {}
// real
"####;
    let reading = lexer::measure(source, Language::Rust);
    assert_eq!(reading.comment_lines, 1, "{reading:?}");
}

#[test]
fn rust_block_comments_nest() {
    let reading = lexer::measure(
        "/* outer\n/* inner */\nstill outer\n*/\nfn after() {}\n",
        Language::Rust,
    );
    assert_eq!(reading.comment_lines, 4);
    assert_eq!(reading.judged_lines, 5);
}

#[test]
fn typescript_regex_templates_and_interpolations_are_distinct() {
    let source = r#"const re = /[/*]/;
const text = `// text ${1 + 2 /* real */}`;
const nested = `x ${`y ${1 /* nested */}`}`;
function f() { return /[/*]/; }
"#;
    let reading = lexer::measure(source, Language::TypeScript);
    assert_eq!(reading.comment_lines, 2, "{reading:?}");
}

#[test]
fn svelte_html_comments_count_but_script_strings_do_not() {
    let reading = lexer::measure(
        "<!-- note -->\n<script>const x = \"// no\";</script>\n<!-- {/* still html */} -->\n",
        Language::Svelte,
    );
    assert_eq!(reading.comment_lines, 2);
}

#[test]
fn only_the_exact_leading_mpl_header_is_excluded() {
    let header = header::notice(Leader::Rust).join("\n");
    let source = format!("{header}\n{header}\n");
    let reading = lexer::measure(&source, Language::Rust);
    assert_eq!(reading.comment_lines, 4);
    assert_eq!(reading.judged_lines, 4);
}

#[test]
fn the_gate_rejects_a_real_fixture_and_accepts_a_clean_one() {
    let root = crate::root::fixture::relocated("comments-gate");
    crate::root::fixture::write(&root, "tools/k/src/a.rs", "// only a comment\n");
    let found = check(&root).unwrap();
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(
        found.first().map(|finding| finding.location.as_str()),
        Some("tools/k/src/a.rs")
    );
    crate::root::fixture::write(&root, "tools/k/src/a.rs", "pub fn clear() {}\n");
    assert!(check(&root).unwrap().is_empty());
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn file_scope_judges_only_the_named_files_and_never_falls_back_to_the_tree() {
    let root = crate::root::fixture::relocated("comments-scope");
    crate::root::fixture::write(&root, "tools/k/src/a.rs", "// debt\n");
    crate::root::fixture::write(&root, "crates/j/src/lib.rs", "// other debt\n");
    let scope = crate::source::Scope::new(&["tools/k/src/a.rs".to_owned()]).unwrap();
    let found = check_scoped(&root, &scope).unwrap();
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(
        found.first().map(|finding| finding.location.as_str()),
        Some("tools/k/src/a.rs")
    );
    let empty = crate::source::Scope::new(&[]).unwrap();
    assert!(check_scoped(&root, &empty).unwrap().is_empty());
    assert!(crate::source::Scope::new(&["../escape.rs".to_owned()]).is_err());
    std::fs::remove_dir_all(&root).unwrap();
}
