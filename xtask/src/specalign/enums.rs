// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The third assertion of `specalign`: every `pub enum` body in
//! `kernel-SPEC.md` holds the variants the kernel compiles, reconciled
//! in both directions.
//!
//! **A SPEC enum body is a roster somebody ports match arms from.** When
//! the SPEC spelled the fourth `SecretCharset` variant `Base36Lower` and
//! the kernel compiled `UpperBase36`, every arm written from the SPEC
//! failed to compile, and nothing in the build said which side was wrong.
//!
//! **An elided body is a pointer, not a roster.** `AxCode` and
//! `EventKind` are written `{ PathNotFound, /* …36 variant */ }`, which
//! states where the roster is rather than what it holds — and that
//! place, the 8-1 and 8-4 tables, is what the other two assertions of
//! this gate reconciles variant by variant. A body carrying `…` is
//! skipped by name, so nobody completes an abbreviation on its behalf.
//!
//! **A SPEC enum the kernel does not compile yet is not a finding.** The
//! SPEC specifies stages that are still planned, and this assertion
//! answers "do these two rosters agree", which needs both of them.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::walk;

/// Where the kernel's compiled enums are read from.
const KERNEL_SRC: &str = "crates/kernel/src";

/// The extension of the files parsed on the code side.
const RUST: [&str; 1] = ["rs"];

/// The opening of a fence this module reads. Prose outside a Rust fence
/// may discuss an enum without declaring one.
const FENCE: &str = "```rust";

/// The marker an abbreviated body carries.
const ELISION: char = '…';
const DECLARATION: &str = "pub enum";

/// One `pub enum` name, as the SPEC's fences declare it. Several cards
/// may declare the same name — a later card amends an earlier one — so
/// the variants are the union of every card, and one elided card marks
/// the whole name as pointing elsewhere.
struct Declared {
    line: usize,
    variants: BTreeSet<String>,
    abbreviated: bool,
}

/// Reconciles the SPEC's enum bodies against the kernel's own enums.
pub(super) fn check(
    root: &Path,
    spec: &str,
    spec_path: &str,
    out: &mut Vec<Violation>,
) -> Result<(), XtaskError> {
    let compiled = compiled(root)?;
    for (name, declared) in declarations(spec) {
        if declared.abbreviated {
            continue;
        }
        let Some(actual) = compiled.get(&name) else {
            continue;
        };
        let at = format!("{spec_path}:{}", declared.line);
        for absent in declared.variants.difference(actual) {
            out.push(mismatch(
                &at,
                format!("`{name}::{absent}` is in the SPEC and not in the kernel"),
                "delete the variant from the SPEC, or add it to the enum — a match arm \
                 written from this body does not compile today",
            ));
        }
        for extra in actual.difference(&declared.variants) {
            out.push(mismatch(
                &at,
                format!("`{name}::{extra}` is in the kernel and not in the SPEC"),
                "add the variant to the SPEC body in the change-set that adds it to the enum",
            ));
        }
    }
    Ok(())
}

fn mismatch(at: &str, violation: String, alternative: &str) -> Violation {
    Violation {
        gate: "specalign",
        location: at.to_owned(),
        rule: "a SPEC enum body lists exactly the variants the kernel compiles \
               (xtask-SPEC.md section 8-10)"
            .to_owned(),
        violation,
        alternative: alternative.to_owned(),
    }
}

/// Every enum the kernel compiles, by name, with its variant names.
/// Enums inside a `#[cfg(test)]` module are left out: a fixture enum is
/// not part of the interface the SPEC settles.
fn compiled(root: &Path) -> Result<BTreeMap<String, BTreeSet<String>>, XtaskError> {
    let mut out = BTreeMap::new();
    for file in walk::files_with_ext(&root.join(KERNEL_SRC), &RUST)? {
        let text = walk::read_text(&file)?;
        let parsed = syn::parse_file(&text).map_err(|err| XtaskError::Doc {
            file: walk::rel(root, &file),
            msg: format!("this file does not parse as Rust: {err}"),
        })?;
        collect(&parsed.items, &mut out);
    }
    Ok(out)
}

fn collect(items: &[syn::Item], out: &mut BTreeMap<String, BTreeSet<String>>) {
    for item in items {
        if let syn::Item::Enum(declared) = item {
            let names = declared.variants.iter().map(|v| v.ident.to_string());
            out.insert(declared.ident.to_string(), names.collect());
        } else if let syn::Item::Mod(module) = item
            && let Some((_, inner)) = &module.content
            && !test_only(&module.attrs)
        {
            collect(inner, out);
        }
    }
}

/// True when this item is compiled for tests only.
fn test_only(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| match &attr.meta {
        syn::Meta::List(list) => {
            list.path.is_ident("cfg") && list.tokens.to_string().contains("test")
        }
        syn::Meta::Path(_) | syn::Meta::NameValue(_) => false,
    })
}

/// The characters of every Rust fence in the SPEC, comments blanked in
/// `code` and kept in `raw`, with the SPEC line each character came
/// from. Both have the same length, so a body located in `code` spans
/// the same characters of `raw` — which is where an elision marker
/// still is, because it lives inside a comment.
struct Fenced {
    code: Vec<char>,
    raw: Vec<char>,
    line: Vec<usize>,
}

fn fenced(spec: &str) -> Fenced {
    let mut raw: Vec<char> = Vec::new();
    let mut line: Vec<usize> = Vec::new();
    let mut inside = false;
    for (index, text) in spec.lines().enumerate() {
        let trimmed = text.trim_start();
        if trimmed.starts_with("```") {
            inside = !inside && trimmed.starts_with(FENCE);
            continue;
        }
        if !inside {
            continue;
        }
        for character in text.chars().chain(std::iter::once('\n')) {
            raw.push(character);
            line.push(index.saturating_add(1));
        }
    }
    let code = blank_comments(&raw);
    Fenced { code, raw, line }
}

/// What the scanner is reading when it reaches a character.
enum Mode {
    Code,
    Line,
    Block,
    Text,
}

/// Replaces every comment character with a space, keeping newlines and
/// keeping the length, so offsets stay comparable with the raw text.
fn blank_comments(raw: &[char]) -> Vec<char> {
    let mut code: Vec<char> = Vec::with_capacity(raw.len());
    let mut mode = Mode::Code;
    let mut index = 0usize;
    while let Some(&character) = raw.get(index) {
        let next = raw.get(index.saturating_add(1)).copied();
        let mut keep = true;
        match mode {
            Mode::Code => {
                if character == '"' {
                    mode = Mode::Text;
                } else if character == '/' && next == Some('/') {
                    mode = Mode::Line;
                    keep = false;
                } else if character == '/' && next == Some('*') {
                    mode = Mode::Block;
                    keep = false;
                }
            }
            Mode::Line => {
                keep = character == '\n';
                if keep {
                    mode = Mode::Code;
                }
            }
            Mode::Block => {
                keep = character == '\n';
                if character == '*' && next == Some('/') {
                    code.push(' ');
                    code.push(' ');
                    index = index.saturating_add(2);
                    mode = Mode::Code;
                    continue;
                }
            }
            Mode::Text => {
                if character == '"' {
                    mode = Mode::Code;
                }
            }
        }
        code.push(if keep { character } else { ' ' });
        index = index.saturating_add(1);
    }
    code
}

/// Every `pub enum` the SPEC's fences declare, merged by name.
fn declarations(spec: &str) -> BTreeMap<String, Declared> {
    let fence = fenced(spec);
    let mut out: BTreeMap<String, Declared> = BTreeMap::new();
    let mut from = 0usize;
    while let Some(start) = find(&fence.code, from, DECLARATION) {
        from = start.saturating_add(1);
        let after = skip_space(&fence.code, start.saturating_add(DECLARATION.len()));
        let (name, end) = ident_at(&fence.code, after);
        let open = skip_space(&fence.code, end);
        if name.is_empty() || fence.code.get(open) != Some(&'{') {
            continue;
        }
        let (Some(close), Some(&line)) = (body_end(&fence.code, open), fence.line.get(start))
        else {
            continue;
        };
        let inner = open.saturating_add(1)..close;
        let (Some(body), Some(source)) = (fence.code.get(inner.clone()), fence.raw.get(inner))
        else {
            continue;
        };
        let entry = out.entry(name).or_insert(Declared {
            line,
            variants: BTreeSet::new(),
            abbreviated: false,
        });
        entry.variants.extend(variants(body));
        entry.abbreviated |= source.contains(&ELISION);
        from = close;
    }
    out
}

/// The variant names of a body, split on the separators this SPEC uses:
/// `,` between variants, and `|` where a card lists them as alternatives.
/// Only depth-zero separators divide, so a struct variant's fields stay
/// inside their variant, and a segment counts only when its first
/// identifier is capitalised — which is what tells a variant from a field.
fn variants(body: &[char]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut depth = 0usize;
    let mut segment = String::new();
    for &character in body {
        if character == '{' || character == '(' || character == '[' {
            depth = depth.saturating_add(1);
        } else if character == '}' || character == ')' || character == ']' {
            // Saturating, so an unbalanced close in prose cannot make the
            // rest of the body look like top level.
            depth = depth.saturating_sub(1);
        } else if depth == 0 && (character == ',' || character == '|') {
            take_variant(&segment, &mut out);
            segment.clear();
            continue;
        }
        if depth == 0 {
            segment.push(character);
        }
    }
    take_variant(&segment, &mut out);
    out
}

fn take_variant(segment: &str, out: &mut BTreeSet<String>) {
    let part = |c: char| c.is_alphanumeric() || c == '_';
    let head = segment.trim_start_matches(|c| !part(c));
    let name: String = head.chars().take_while(|c| part(*c)).collect();
    if name.chars().next().is_some_and(char::is_uppercase) {
        out.insert(name);
    }
}

fn find(code: &[char], from: usize, needle: &str) -> Option<usize> {
    let pattern: Vec<char> = needle.chars().collect();
    let tail = code.get(from..)?;
    let at = tail.windows(pattern.len()).position(|w| w == pattern)?;
    from.checked_add(at)
}

fn skip_space(code: &[char], from: usize) -> usize {
    let mut index = from;
    while code.get(index).is_some_and(|c| c.is_whitespace()) {
        index = index.saturating_add(1);
    }
    index
}

fn ident_at(code: &[char], from: usize) -> (String, usize) {
    let mut name = String::new();
    let mut index = from;
    while let Some(&character) = code.get(index) {
        if character.is_alphanumeric() || character == '_' {
            name.push(character);
            index = index.saturating_add(1);
        } else {
            break;
        }
    }
    (name, index)
}

/// The brace closing the one at `open`, or None when the fence ends.
fn body_end(code: &[char], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut index = open;
    while let Some(&character) = code.get(index) {
        if character == '{' {
            depth = depth.checked_add(1)?;
        } else if character == '}' {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
        index = index.checked_add(1)?;
    }
    None
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::{declarations, variants};

    #[test]
    fn a_body_splits_on_commas_and_on_alternatives() {
        let body: Vec<char> = "Text{text} | ToolUse{id, name: ToolName} | Lines { from: u64 }"
            .chars()
            .collect();
        let found = variants(&body);
        assert_eq!(found.len(), 3);
        assert!(found.contains("ToolUse"));
        assert!(!found.contains("name"));
    }

    #[test]
    fn only_rust_fences_declare_and_a_later_card_amends_an_earlier_one() {
        let spec = "prose naming pub enum Ghost { Absent }\n\
                    ```rust\npub enum Verdict { Within }\n```\n\
                    ```text\npub enum Fake { Nope }\n```\n\
                    ```rust\npub enum Verdict { Outside { prefixes: Vec<String> } }\n```\n";
        let found = declarations(spec);
        assert!(!found.contains_key("Ghost"));
        assert!(!found.contains_key("Fake"));
        let verdict = found.get("Verdict").unwrap();
        assert_eq!(verdict.variants.len(), 2);
        assert!(verdict.variants.contains("Outside"));
        assert!(!verdict.abbreviated);
    }

    #[test]
    fn an_elided_body_points_elsewhere_and_is_not_a_roster() {
        let spec = "```rust\npub enum AxCode { PathNotFound, /* …36 variant */ }\n```\n";
        let found = declarations(spec);
        let code = found.get("AxCode").unwrap();
        assert!(code.abbreviated);
        assert_eq!(code.variants.len(), 1);
    }

    #[test]
    fn a_comment_neither_declares_a_variant_nor_closes_the_body() {
        let spec = "```rust\npub enum Range {\n    // Bytes is described, not declared }\n\
                    Lines { from: u64 },\n}\n```\n";
        let range = declarations(spec).remove("Range").unwrap();
        assert_eq!(range.variants.len(), 1);
        assert!(range.variants.contains("Lines"));
    }
}
