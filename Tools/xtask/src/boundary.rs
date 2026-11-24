// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use syn::spanned::Spanned;

use crate::budget::REGISTER;
use crate::members::{self, Member, Role};
use crate::report::{Violation, XtaskError};
use crate::walk;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Reviewed {
    file: String,
    statement: String,
    reason: String,
}

const EXEMPT_MARK: &str = "boundary-ok:";

const CROSSINGS: [(&str, &str); 5] = [
    (
        "CARGO_BIN_EXE",
        "cargo's own handle on the binary this repository builds",
    ),
    (
        "SPRAWLING_BIN",
        "the path `just adversary` hands the shipped binary over",
    ),
    (
        "axum::serve",
        "raising this product's own HTTP surface in order to talk to it over a socket",
    ),
    (
        "tungstenite",
        "a WebSocket client, and the only WebSocket server in this world is ours",
    ),
    (
        "assert_cmd",
        "a command-line harness, which exists to drive a built binary",
    ),
];

const FUZZ: &str = "Tools/fuzz/";
const GATES: &str = env!("CARGO_PKG_NAME");

#[derive(Debug, PartialEq, Eq)]
enum Reading {
    Whole,
    Marked,
    Skipped,
}

fn reading(rel: &str, found: &[Member]) -> Reading {
    if rel.starts_with(FUZZ) {
        return Reading::Whole;
    }
    let Some(owner) = members::owner(found, rel) else {
        return Reading::Skipped;
    };
    match owner.role {
        Role::Tool if owner.package != GATES => Reading::Whole,
        Role::Tool | Role::Product if rel.contains("/tests/") => Reading::Whole,
        Role::Tool | Role::Product => Reading::Marked,
    }
}

fn test_spans(text: &str, rel: &str) -> Result<Vec<(usize, usize)>, XtaskError> {
    let parsed = syn::parse_file(text).map_err(|err| XtaskError::Doc {
        file: rel.to_owned(),
        msg: format!("this file does not parse as Rust: {err}"),
    })?;
    let mut spans = Vec::new();
    collect(&parsed.items, &mut spans);
    Ok(spans)
}

fn collect(items: &[syn::Item], out: &mut Vec<(usize, usize)>) {
    for item in items {
        if marked_test(attrs_of(item)) {
            let span = item.span();
            out.push((span.start().line, span.end().line));
            continue;
        }
        if let syn::Item::Mod(module) = item
            && let Some((_, inner)) = &module.content
        {
            collect(inner, out);
        }
    }
}

fn attrs_of(item: &syn::Item) -> &[syn::Attribute] {
    match item {
        syn::Item::Fn(it) => &it.attrs,
        syn::Item::Mod(it) => &it.attrs,
        syn::Item::Impl(it) => &it.attrs,
        syn::Item::Struct(it) => &it.attrs,
        syn::Item::Enum(it) => &it.attrs,
        syn::Item::Use(it) => &it.attrs,
        syn::Item::Const(it) => &it.attrs,
        syn::Item::Static(it) => &it.attrs,
        syn::Item::Trait(it) => &it.attrs,
        syn::Item::Type(it) => &it.attrs,
        _ => &[],
    }
}

fn marked_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("cfg")
            && attr
                .meta
                .require_list()
                .is_ok_and(|list| list.tokens.to_string().contains("test"))
    })
}

const ROW: &str = "boundary";
const PREDATING: &str = "predating";

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let register = crate::budget::register(root)?;
    let excused = excused(&register)?;
    let mut reviewed: Vec<Reviewed> = match register.get(ROW).and_then(|row| row.get("reviewed")) {
        None => Vec::new(),
        Some(rows) => rows
            .clone()
            .try_into()
            .map_err(|err: toml::de::Error| XtaskError::Doc {
                file: REGISTER.to_owned(),
                msg: err.to_string(),
            })?,
    };
    let found = members::members(root)?;
    let mut crossing = std::collections::BTreeSet::new();
    let mut violations = Vec::new();
    for file in walk::files_with_ext(root, &["rs"])? {
        let rel = walk::rel(root, &file);
        if walk::in_isolation_zone(&rel) || rel == "Tools/xtask/src/boundary.rs" {
            continue;
        }
        let whole = match reading(&rel, &found) {
            Reading::Skipped => continue,
            Reading::Whole => true,
            Reading::Marked => false,
        };
        let text = walk::read_text(&file)?;
        let spans = if whole {
            Vec::new()
        } else {
            test_spans(&text, &rel)?
        };
        if !whole && spans.is_empty() {
            continue;
        }
        let mut found = Vec::new();
        Sweep {
            rel: &rel,
            text: &text,
            whole,
            spans: &spans,
        }
        .scan(&mut found, &mut reviewed);
        if found.is_empty() {
            continue;
        }
        crossing.insert(rel.clone());
        if !excused.contains(&rel) {
            violations.extend(found);
        }
    }
    for row in reviewed {
        violations.push(Violation {
            gate: "boundary",
            location: row.file,
            rule: "a reviewed statement must correspond to exactly one observed crossing with a reason".to_owned(),
            violation: row.statement,
            alternative: format!("remove or correct the reviewed statement in {REGISTER}"),
        });
    }
    for spent in excused.difference(&crossing) {
        violations.push(Violation {
            gate: "boundary",
            location: format!("{REGISTER} [{ROW}.{PREDATING}]"),
            rule: "an exception that is no longer needed is struck from the register".to_owned(),
            violation: format!("{spent} no longer crosses the process boundary"),
            alternative: "delete its row: the check has moved to Tools/adversary/ or come back \
                          in-process, and the debt it recorded is paid"
                .to_owned(),
        });
    }
    Ok(violations)
}

fn excused(register: &toml::Value) -> Result<std::collections::BTreeSet<String>, XtaskError> {
    let Some(listed) = register
        .get(ROW)
        .and_then(|row| row.get(PREDATING))
        .and_then(|table| table.get("files"))
        .and_then(toml::Value::as_array)
    else {
        return Ok(std::collections::BTreeSet::new());
    };
    let mut out = std::collections::BTreeSet::new();
    for value in listed {
        let named = value.as_str().ok_or_else(|| XtaskError::Doc {
            file: REGISTER.to_owned(),
            msg: format!("{ROW}.{PREDATING} holds something that is not a path"),
        })?;
        out.insert(named.to_owned());
    }
    Ok(out)
}

struct Sweep<'a> {
    rel: &'a str,
    text: &'a str,
    whole: bool,
    spans: &'a [(usize, usize)],
}

impl Sweep<'_> {
    fn scan(&self, out: &mut Vec<Violation>, reviewed: &mut Vec<Reviewed>) {
        let mut previous_exempts = false;
        for (index, line) in self.text.lines().enumerate() {
            let number = index.saturating_add(1);
            let marked = line.contains(EXEMPT_MARK);
            let exempt = marked || previous_exempts;
            previous_exempts = marked;
            if exempt || !self.is_test_code(number) {
                continue;
            }
            for (token, what) in CROSSINGS {
                if line.contains(token) {
                    if let Some(index) = reviewed.iter().position(|row| {
                        row.file == self.rel
                            && row.statement == line.trim()
                            && !row.reason.trim().is_empty()
                    }) {
                        reviewed.swap_remove(index);
                    } else {
                        out.push(crossed(self.rel, number, token, what));
                    }
                    break;
                }
            }
        }
    }

    fn is_test_code(&self, line: usize) -> bool {
        self.whole
            || self
                .spans
                .iter()
                .any(|(first, last)| line >= *first && line <= *last)
    }
}

fn crossed(rel: &str, line: usize, token: &str, what: &str) -> Violation {
    Violation {
        gate: "boundary",
        location: format!("{rel}:{line}"),
        rule: "a check that crosses the process boundary is written in Lean under \
               Tools/adversary/; Rust checks link the crates and enter by their public faces"
            .to_owned(),
        violation: format!("`{token}` in test code: {what}"),
        alternative: "move it to Tools/adversary/ as a property over traces, or drive the same \
                      thing in-process through the door wire::server uses; if this is not \
                      this product's boundary, narrow CROSSINGS to the exact external surface"
            .to_owned(),
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    fn owner(rel: &str) -> Member {
        Member {
            package: "product".to_owned(),
            lib: None,
            dir: rel.split('/').take(2).collect::<Vec<_>>().join("/"),
            role: Role::Product,
            depends_on: std::collections::BTreeSet::new(),
            publish: crate::members::Publish::Registry,
            roots: Vec::new(),
        }
    }

    fn found(rel: &str, source: &str) -> Vec<Violation> {
        let mut out = Vec::new();
        let spans = test_spans(source, rel).unwrap();
        Sweep {
            rel,
            text: source,
            whole: reading(rel, &[owner(rel)]) == Reading::Whole,
            spans: &spans,
        }
        .scan(&mut out, &mut Vec::new());
        out
    }

    #[test]
    fn a_socket_to_a_double_the_test_raised_is_not_a_crossing() {
        let source = "\
            #[cfg(test)]\n\
            mod tests {\n\
                fn hostile_upstream() {\n\
                    let (url, _p) = fake_openai(&[\"m\"], vec![]);\n\
                    drop(std::net::TcpStream::connect(&url).unwrap());\n\
                }\n\
            }\n";
        assert!(found("Sprawling/accounting/src/worker/driving.rs", source).is_empty());
    }

    #[test]
    fn driving_the_built_binary_is_refused() {
        let source = "\
            fn run() {}\n\
            #[cfg(test)]\n\
            mod tests {\n\
                #[test]\n\
                fn drives_the_binary() {\n\
                    let _ = std::process::Command::new(env!(\"CARGO_BIN_EXE_sprawling\"));\n\
                }\n\
            }\n";
        let out = found("Sprawling/app/src/main.rs", source);
        assert_eq!(out.len(), 1, "{out:#?}");
        assert!(out[0].location.ends_with(":6"), "{:?}", out[0].location);
    }

    #[test]
    fn a_file_under_tests_is_test_code_by_where_it_lives() {
        let source = "fn helper() {\n    let _ = axum::serve(listener, app);\n}\n";
        assert_eq!(found("Sprawling/wire/tests/enrolment.rs", source).len(), 1);
        assert!(found("Sprawling/wire/src/server.rs", source).is_empty());
    }

    #[test]
    fn each_review_matches_one_statement_in_one_file_with_a_reason() {
        let path = "Sprawling/wire/tests/sample.rs";
        let line = "let _ = axum::serve(listener, app);";
        let repeated = format!("{line}\n{line}\n");
        for (file, statement, reason, consumed) in [
            (path, line, "a loopback listener", true),
            (
                "Sprawling/wire/tests/other.rs",
                line,
                "a loopback listener",
                false,
            ),
            (
                path,
                "let _ = axum::serve(other, app);",
                "a loopback listener",
                false,
            ),
            (path, line, " ", false),
        ] {
            let mut reviewed = vec![Reviewed {
                file: file.into(),
                statement: statement.into(),
                reason: reason.into(),
            }];
            let mut violations = Vec::new();
            Sweep {
                rel: path,
                text: &repeated,
                whole: true,
                spans: &[],
            }
            .scan(&mut violations, &mut reviewed);
            assert_eq!(violations.len(), if consumed { 1 } else { 2 });
            assert_eq!(reviewed.is_empty(), consumed);
        }
    }

    #[test]
    fn a_waiver_on_the_line_or_the_line_above_is_honoured() {
        let source = "\
            #[cfg(test)]\n\
            mod tests {\n\
                fn a() {\n\
                    // boundary-ok: a reason\n\
                    let _ = env!(\"CARGO_BIN_EXE_sprawling\");\n\
                }\n\
                fn b() {\n\
                    let _ = env!(\"CARGO_BIN_EXE_sprawling\"); // boundary-ok: same\n\
                }\n\
            }\n";
        assert!(found("Sprawling/runtime/src/x.rs", source).is_empty());
    }
}
