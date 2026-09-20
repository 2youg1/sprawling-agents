// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `cargo xtask docnum [--write]`: the numbers a document quotes, taken
//! from the code that decides them.
//!
//! A document may mark a span as managed by a fact:
//!
//! ```text
//! <!-- xtask:begin gate_count -->
//! 20
//! <!-- xtask:end -->
//! ```
//!
//! The gate recounts every fact and refuses a span whose text is not
//! what the recount says; `--write` replaces the text instead. Markers
//! sitting on their own lines wrap the value in newlines, and markers
//! written around a value on one line keep it on one line, so a table
//! cell and a paragraph can both hold a managed span.
//!
//! A key may carry one argument after a colon — `dep_version:toml`,
//! `budget_reading:frontend_artifact` — so one generator serves a family
//! of facts.
//!
//! The `facts` module is the authority for which facts a document may
//! quote. Every entry names the code it recounts from, so a number in a
//! document has the same home as the number in the binary — the defect
//! this gate closes is a document that stated `WIRE_V` as 15 while the
//! wire had reached 31, which the next reader (a person or a model)
//! writes code against.

use std::collections::BTreeMap;
use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::walk;

mod facts;

/// What opens a managed span, up to the fact's key.
const BEGIN: &str = "<!-- xtask:begin ";

/// What closes the opening marker, after the key.
const OPENED: &str = " -->";

/// What closes a managed span.
const END: &str = "<!-- xtask:end -->";

/// Every reading this run has already taken, so a fact scanning the
/// whole tree is recounted once however many documents quote it. A key
/// no fact owns is absent, which is what a refusal reports.
type Readings = BTreeMap<String, String>;

/// Today's reading for every key these spans name.
fn readings(root: &Path, found: &[Span], into: &mut Readings) -> Result<(), XtaskError> {
    for span in found {
        if into.contains_key(&span.key) {
            continue;
        }
        if let Some(value) = facts::value(root, &span.key)? {
            into.insert(span.key.clone(), value);
        }
    }
    Ok(())
}

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut values = Readings::new();
    let mut violations = Vec::new();
    for file in documents(root)? {
        let rel = walk::rel(root, &file);
        let text = walk::read_text(&file)?;
        match spans(&text) {
            Err(malformed) => violations.push(malformed.violation(&rel)),
            Ok(found) => {
                readings(root, &found, &mut values)?;
                for span in found {
                    judge(&rel, &span, &values, &mut violations);
                }
            }
        }
    }
    Ok(violations)
}

/// `cargo xtask docnum --write`: every managed span rewritten to what
/// its fact says today. A span naming no fact stops the write, because
/// a writer that skips what it cannot recount leaves the document
/// looking regenerated while one number stays stale.
pub(crate) fn write(root: &Path) -> Result<String, XtaskError> {
    let mut values = Readings::new();
    let mut changed = 0_usize;
    for file in documents(root)? {
        let rel = walk::rel(root, &file);
        let text = walk::read_text(&file)?;
        let found = spans(&text).map_err(|malformed| XtaskError::Doc {
            file: format!("{rel}:{}", malformed.line),
            msg: malformed.why,
        })?;
        readings(root, &found, &mut values)?;
        let rewritten = rewrite(&text, &found, &values, &rel)?;
        if rewritten == text {
            continue;
        }
        std::fs::write(&file, &rewritten).map_err(|source| XtaskError::Io {
            path: rel.clone(),
            source,
        })?;
        changed = changed.saturating_add(1);
        println!("written: {rel}");
    }
    Ok(format!("docnum: {changed} document(s) rewritten\n"))
}

/// Every Markdown file a reader can receive. The isolation zone holds
/// one machine's working notes and is never published, so a marker
/// there is nobody's authority.
fn documents(root: &Path) -> Result<Vec<std::path::PathBuf>, XtaskError> {
    let all = walk::files_with_ext(root, &["md"])?;
    Ok(all
        .into_iter()
        .filter(|path| !walk::in_isolation_zone(&walk::rel(root, path)))
        .collect())
}

/// One managed span: the key it names, where it starts, and the text
/// between the two markers.
#[derive(Debug)]
struct Span {
    key: String,
    line: usize,
    /// Byte range of the text between the markers.
    from: usize,
    to: usize,
    content: String,
}

impl Span {
    /// What this span should read, in the shape its author chose: a
    /// value on its own line when the markers are, inline when they
    /// are.
    fn framed(&self, value: &str) -> String {
        if self.content.contains('\n') {
            format!("\n{value}\n")
        } else {
            value.to_owned()
        }
    }
}

/// A marker pair this module cannot read, and where it starts.
#[derive(Debug)]
struct Malformed {
    line: usize,
    why: String,
}

impl Malformed {
    fn violation(self, rel: &str) -> Violation {
        Violation {
            gate: "docnum",
            location: format!("{rel}:{}", self.line),
            rule: "a managed span opens with `<!-- xtask:begin <fact> -->` and closes with `<!-- xtask:end -->`".to_owned(),
            violation: self.why,
            alternative: "close the span, or delete both markers and quote nothing".to_owned(),
        }
    }
}

fn judge(rel: &str, span: &Span, values: &Readings, out: &mut Vec<Violation>) {
    let Some(value) = values.get(&span.key) else {
        out.push(Violation {
            gate: "docnum",
            location: format!("{rel}:{}", span.line),
            rule: "a managed span names a fact the generator array defines".to_owned(),
            violation: format!("`{}` is not a fact xtask can recount", span.key),
            alternative: format!(
                "add it to `FACTS` in xtask/src/docnum/facts.rs, or name one of: {}",
                facts::keys()
            ),
        });
        return;
    };
    let expected = span.framed(value);
    if span.content == expected {
        return;
    }
    out.push(Violation {
        gate: "docnum",
        location: format!("{rel}:{}", span.line),
        rule: "a number in a document is the number its code holds".to_owned(),
        violation: format!(
            "this span reads `{}` where {} says `{value}`",
            span.content.trim(),
            facts::home(&span.key)
        ),
        alternative: "run `cargo xtask docnum --write` and commit the result".to_owned(),
    });
}

/// Every managed span in one document, in the order they appear.
fn spans(text: &str) -> Result<Vec<Span>, Malformed> {
    let mut out: Vec<Span> = Vec::new();
    let mut cursor = 0_usize;
    while let Some(offset) = text.get(cursor..).and_then(|rest| rest.find(BEGIN)) {
        let opens = cursor.saturating_add(offset);
        let after_begin = opens.saturating_add(BEGIN.len());
        let rest = text.get(after_begin..).unwrap_or_default();
        let Some(key_len) = rest.find(OPENED) else {
            return Err(Malformed {
                line: line_of(text, opens),
                why: "this begin marker never closes its `-->`".to_owned(),
            });
        };
        let key = rest.get(..key_len).unwrap_or_default().trim().to_owned();
        let from = after_begin
            .saturating_add(key_len)
            .saturating_add(OPENED.len());
        let tail = text.get(from..).unwrap_or_default();
        let Some(closes) = tail.find(END) else {
            return Err(Malformed {
                line: line_of(text, opens),
                why: format!("the span named `{key}` has no `{END}` after it"),
            });
        };
        let content = tail.get(..closes).unwrap_or_default().to_owned();
        if content.contains(BEGIN) {
            return Err(Malformed {
                line: line_of(text, opens),
                why: format!("the span named `{key}` opens a second span before it closes"),
            });
        }
        cursor = from.saturating_add(closes).saturating_add(END.len());
        out.push(Span {
            key,
            line: line_of(text, opens),
            from,
            to: from.saturating_add(closes),
            content,
        });
    }
    let closers = text.matches(END).count();
    if closers != out.len() {
        return Err(Malformed {
            line: 1,
            why: format!(
                "{closers} end marker(s) and {} span(s): one closes nothing",
                out.len()
            ),
        });
    }
    Ok(out)
}

/// The document with every managed span replaced by today's reading.
fn rewrite(text: &str, found: &[Span], values: &Readings, rel: &str) -> Result<String, XtaskError> {
    let mut out = String::with_capacity(text.len());
    let mut done = 0_usize;
    for span in found {
        let Some(value) = values.get(&span.key) else {
            return Err(XtaskError::Doc {
                file: format!("{rel}:{}", span.line),
                msg: format!(
                    "`{}` is not a fact xtask can recount; known facts: {}",
                    span.key,
                    facts::keys()
                ),
            });
        };
        out.push_str(text.get(done..span.from).unwrap_or_default());
        out.push_str(&span.framed(value));
        done = span.to;
    }
    out.push_str(text.get(done..).unwrap_or_default());
    Ok(out)
}

/// The one-based line a byte offset sits on.
fn line_of(text: &str, offset: usize) -> usize {
    text.get(..offset)
        .unwrap_or_default()
        .matches('\n')
        .count()
        .saturating_add(1)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
