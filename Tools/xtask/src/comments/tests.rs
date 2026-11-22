// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(clippy::unwrap_used, reason = "test code")]

use super::*;
use crate::header::{self, Leader};

#[test]
fn only_the_exact_mpl_header_is_admitted() {
    let mut source = header::notice(Leader::Rust).join("\n");
    source.push('\n');
    assert_eq!(lexer::measure(&source, Language::Rust).comment_lines, 0);
    source.push_str("// forbidden\n");
    assert_eq!(lexer::measure(&source, Language::Rust).comment_lines, 1);
}

#[test]
fn an_inline_comment_counts_and_an_empty_file_is_clean() {
    let reading = lexer::measure(
        "const FIRST: u8 = 1; // why\nconst SECOND: u8 = 2;\n",
        Language::TypeScript,
    );
    assert_eq!(reading.comment_lines, 1);
    assert_eq!(lexer::measure("", Language::Rust).comment_lines, 0);
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
fn svelte_block_expressions_use_javascript_lexing() {
    let source = r#"{#if "}" /* first */ ready}
<p>ok</p>
{:else if other /* second */}
<p>other</p>
{/if}
<!-- after -->
"#;
    let reading = lexer::measure(source, Language::Svelte);
    assert_eq!(reading.comment_lines, 3, "{reading:?}");
    let data = r#"{#if ready}
<p>ok</p>
{:else if "/* data */"}
<p>other</p>
{/if}
"#;
    assert_eq!(lexer::measure(data, Language::Svelte).comment_lines, 0);
}

#[test]
fn svelte_script_open_tags_may_span_lines() {
    let source =
        "<script\n  lang=\"ts\"\n>\n  // first\n  // second\n  const value = 1;\n</script>\n";
    let reading = lexer::measure(source, Language::Svelte);
    assert_eq!(reading.comment_lines, 2, "{reading:?}");
}

#[test]
fn javascript_line_comments_after_colons_count() {
    let source = "const value = { key:// reason\n    1 };\n";
    let reading = lexer::measure(source, Language::TypeScript);
    assert_eq!(reading.comment_lines, 1, "{reading:?}");
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

#[test]
fn zig_multiline_strings_do_not_hide_real_comments() {
    let source = r#"const text =
    \\// not a comment
    \\/* also text */
    ;
// real
"#;
    assert_eq!(lexer::measure(source, Language::Zig).comment_lines, 1);
}

#[test]
fn python_strings_and_hashes_are_distinct() {
    let source = r##"#!/usr/bin/env python3
text = "# not a comment"
triple = """
# also text
"""
# real
"##;
    assert_eq!(lexer::measure(source, Language::Python).comment_lines, 2);
    let header = header::NOTICE.map(|row| format!("# {row}")).join("\n");
    assert_eq!(
        lexer::measure(&format!("{header}\n# real\n"), Language::Python).comment_lines,
        1
    );
}

#[test]
fn python_fstring_expression_comments_reach_the_real_gate() {
    let source = "value = f\"\"\"{1 # forbidden\n}\"\"\"\n";
    assert_eq!(lexer::measure(source, Language::Python).comment_lines, 1);
    let root = crate::root::fixture::relocated("comments-python-fstring");
    crate::root::fixture::write(&root, "tools/k/src/fstring.py", source);
    assert_eq!(check(&root).unwrap().len(), 1);
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn python_fstring_format_specifiers_are_not_comments() {
    let source = "value = f\"{42:#x}\"\n";
    assert_eq!(lexer::measure(source, Language::Python).comment_lines, 0);
    let root = crate::root::fixture::relocated("comments-python-format");
    crate::root::fixture::write(&root, "tools/k/src/format.py", source);
    assert!(check(&root).unwrap().is_empty());
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn python_raw_fstring_backslashes_preserve_expression_boundaries() {
    let source = "value = rf\"\"\"\\{1 # forbidden\n}\"\"\"\n";
    assert_eq!(lexer::measure(source, Language::Python).comment_lines, 1);
    let root = crate::root::fixture::relocated("comments-python-raw-fstring");
    crate::root::fixture::write(&root, "tools/k/src/raw.py", source);
    assert_eq!(check(&root).unwrap().len(), 1);
    std::fs::remove_file(root.join("tools/k/src/raw.py")).unwrap();
    let escaped = "value = rf\"\\\"\" # forbidden\n";
    assert_eq!(lexer::measure(escaped, Language::Python).comment_lines, 1);
    crate::root::fixture::write(&root, "tools/k/src/escaped.py", escaped);
    assert_eq!(check(&root).unwrap().len(), 1);
    std::fs::remove_file(root.join("tools/k/src/escaped.py")).unwrap();
    let data = r#"value = f"\N{LATIN SMALL LETTER A}"
quoted = f"\""
braces = f"{{# data}}"
"#;
    crate::root::fixture::write(&root, "tools/k/src/data.py", data);
    assert!(check(&root).unwrap().is_empty());
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn css_mpl_and_strings_leave_only_the_real_comment() {
    let source = "/* This Source Code Form is subject to the terms of the Mozilla Public\n * License, v. 2.0. If a copy of the MPL was not distributed with this\n * file, You can obtain one at https://mozilla.org/MPL/2.0/.\n * Copyright (c) 2026 2youg1 and the sprawling contributors */\n.a::after { content: \"/* not a comment */\"; }\n/* real */\n";
    assert_eq!(lexer::measure(source, Language::Css).comment_lines, 1);
}

#[test]
fn svelte_style_uses_css_and_script_uses_javascript() {
    let source = r#"<style>.a::after { content: "// text"; }</style>
<style>.b::after { content: "/* text */"; }</style>
<style>/* real css */</style>
<script>// real script</script>
"#;
    assert_eq!(lexer::measure(source, Language::Svelte).comment_lines, 2);
}
