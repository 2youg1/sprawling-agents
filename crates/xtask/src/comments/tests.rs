// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(clippy::unwrap_used, reason = "test code")]

use super::*;
use crate::header::{self, Leader};

#[test]
fn the_boundary_is_ten_percent_and_one_more_comment_is_rejected() {
    let mut source = header::notice(Leader::Rust).join("\n");
    source.push('\n');
    source.push_str(&"// necessary contract\n".repeat(10));
    source.push_str(&"const VALUE: u8 = 1;\n".repeat(90));
    let accepted = lexer::measure(&source, Language::Rust);
    assert_eq!(
        accepted,
        Reading {
            comment_lines: 10,
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
fn regex_after_paren_keeps_character_class_out_of_block_comments() {
    let reading = lexer::measure("if (ready) /[/*]/.test(value);\n", Language::TypeScript);
    assert_eq!(reading.comment_lines, 0, "{reading:?}");
}

#[test]
fn division_after_increment_still_sees_a_trailing_comment() {
    let reading = lexer::measure(
        "const ratio = count++ / divisor; // note\n",
        Language::TypeScript,
    );
    assert_eq!(reading.comment_lines, 1, "{reading:?}");
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
fn markup_urls_are_not_line_comments() {
    let svelte = lexer::measure("<p>https://example.com</p>\n", Language::Svelte);
    assert_eq!(svelte.comment_lines, 0, "{svelte:?}");
    let tsx = lexer::measure(
        "export function Page() { return <p>https://example.com</p>; }\n",
        Language::TypeScript,
    );
    assert_eq!(tsx.comment_lines, 0, "{tsx:?}");
}

#[test]
fn svelte_control_blocks_do_not_swallow_following_comments() {
    let source = "{#if ready}\n<p>ok</p>\n{/if}\n<!-- note -->\n";
    let reading = lexer::measure(source, Language::Svelte);
    assert_eq!(reading.comment_lines, 1, "{reading:?}");
}

#[test]
fn svelte_script_open_tags_may_span_lines() {
    let source =
        "<script\n  lang=\"ts\"\n>\n  // first\n  // second\n  const value = 1;\n</script>\n";
    let reading = lexer::measure(source, Language::Svelte);
    assert_eq!(reading.comment_lines, 2, "{reading:?}");
}

#[test]
fn rust_keeps_a_line_comment_after_a_colon() {
    let source = "struct Item { value: u8 }\nfn item() -> Item {\n    Item { value:// rationale\n        0 }\n}\n";
    let reading = lexer::measure(source, Language::Rust);
    assert_eq!(reading.comment_lines, 1, "{reading:?}");
}

#[test]
fn division_after_index_still_sees_a_block_comment() {
    let reading = lexer::measure(
        "const ratio = values[0] / divisor; /* note */\n",
        Language::TypeScript,
    );
    assert_eq!(reading.comment_lines, 1, "{reading:?}");
}

#[test]
fn svelte_mpl_headers_are_excluded_in_every_leading_shape() {
    let wrap =
        "<!--\n".to_owned() + &header::notice(Leader::Rust).join("\n") + "\n-->\n<p>ok</p>\n";
    assert_eq!(
        lexer::measure(&wrap, Language::Svelte).comment_lines,
        0,
        "{wrap}"
    );
    let inline = format!("<!-- {} -->\n<p>ok</p>\n", header::NOTICE.join("\n     "));
    assert_eq!(
        lexer::measure(&inline, Language::Svelte).comment_lines,
        0,
        "{inline}"
    );
    let script = format!(
        "<script lang=\"ts\">\n  {}\n  const x = 1;\n</script>\n",
        header::notice(Leader::Rust).join("\n  ")
    );
    assert_eq!(
        lexer::measure(&script, Language::Svelte).comment_lines,
        0,
        "{script}"
    );
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
