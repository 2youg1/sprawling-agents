// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a bundle or a playback page is found to be, as five separate
//! items (accounting-SPEC.md 8-12 and 8-13, decision 25(g)).
//!
//! A bundle on its own can only be found consistent: that says nothing
//! about whether it was changed, because a changed bundle can be made
//! consistent again. Against another bundle it is the same bytes or not.
//! Against its city it is recomputed with the reader the caller names
//! and compared whole, so keeping `source` while changing a table does
//! not pass. A page adds what it loads and what a browser saw it do; each
//! of these is its own item, because a static pass and an observed
//! browser are different claims.

use std::collections::BTreeSet;
use std::path::Path;

use kernel::{AxError, B3Hash, Seq};
use serde_json::{Map, Value};

use super::consistency::consistent;
use super::document::{Decimal, Document};
use super::encode::{Bundle, decode};
use super::page::Page;
use super::reader::Reader;
use super::select::Selection;
use super::{Cutoff, PROJECTION_RULES, Projected, Request, observed, offline, project};

/// The city a bundle is recomputed from, and who reads it there.
#[derive(Debug, Clone)]
pub struct City<'a> {
    pub root: &'a Path,
    pub reader: Reader,
}

/// What a check is asked to compare the file with, beyond itself.
#[derive(Debug, Clone, Default)]
pub struct Asked<'a> {
    /// Another bundle's bytes.
    pub bundle: Option<&'a [u8]>,
    /// The city the file's bundle was exported from.
    pub city: Option<City<'a>>,
    /// What a browser saw the page do, as the skill recorded it.
    pub observed: Option<&'a [u8]>,
}

/// One item's conclusion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Passed,
    Failed {
        found: String,
    },
    /// Nobody asked for this item, or the file is not one it applies to.
    Unasked {
        why: &'static str,
    },
    /// It was asked for and could not be done.
    Unable {
        why: String,
    },
}

/// The five items, and the bundle they are about when one was read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub digest: Option<B3Hash>,
    pub events: Option<usize>,
    pub structure: Verdict,
    pub bundle: Verdict,
    pub source: Verdict,
    pub offline: Verdict,
    pub browser: Verdict,
    /// The paths the browser walked, when its item passed.
    pub covered: Vec<String>,
}

impl Report {
    /// No item failed, and none that was asked for went undone.
    #[must_use]
    pub fn holds(&self) -> bool {
        self.items()
            .iter()
            .all(|(_, verdict)| matches!(verdict, Verdict::Passed | Verdict::Unasked { .. }))
    }

    /// The report as one JSON object: the digest and event count when a
    /// bundle was read, then each item as `passed`, `failed` with what
    /// was found, or `unchecked` with why.
    #[must_use]
    pub fn line(&self) -> Value {
        let mut line = Map::new();
        if let Some(digest) = self.digest {
            line.insert("digest".to_owned(), digest.to_string().into());
        }
        if let Some(events) = self.events {
            line.insert("events".to_owned(), events.to_string().into());
        }
        for (name, verdict) in self.items() {
            let mut item = Map::new();
            let (status, detail) = match verdict {
                Verdict::Passed => ("passed", None),
                Verdict::Failed { found } => ("failed", Some(("found", found.clone()))),
                Verdict::Unasked { why } => ("unchecked", Some(("why", (*why).to_owned()))),
                Verdict::Unable { why } => ("unchecked", Some(("why", why.clone()))),
            };
            item.insert("status".to_owned(), status.into());
            if let Some((key, text)) = detail {
                item.insert(key.to_owned(), text.into());
            }
            if name == "browser" && *verdict == Verdict::Passed {
                item.insert("covered".to_owned(), self.covered.clone().into());
            }
            line.insert(name.to_owned(), Value::Object(item));
        }
        Value::Object(line)
    }

    fn items(&self) -> [(&'static str, &Verdict); 5] {
        [
            ("structure", &self.structure),
            ("bundle", &self.bundle),
            ("source", &self.source),
            ("offline", &self.offline),
            ("browser", &self.browser),
        ]
    }
}

/// Checks `file`, a bundle or a playback page, item by item. A file whose
/// first byte is `{` is a bundle; anything else is a page.
#[must_use]
pub fn check(file: &[u8], asked: &Asked<'_>) -> Report {
    let (opened, offline) = match file.first() {
        Some(b'{') => (
            opened(file, None),
            Verdict::Unasked {
                why: "a bundle loads nothing; the offline check reads a page",
            },
        ),
        Some(_) | None => match std::str::from_utf8(file) {
            Ok(text) => {
                let page = Page::read(text);
                (
                    page.bundle_text()
                        .and_then(|bundle| opened(bundle.as_bytes(), Some(&page))),
                    offline_verdict(offline::findings(&page)),
                )
            }
            Err(err) => (
                Err(format!("the page is not UTF-8: {err}")),
                Verdict::Unable {
                    why: "a page that is not UTF-8 is not read".to_owned(),
                },
            ),
        },
    };
    let (browser, covered) = match asked.observed {
        Some(record) => observed::judge(record, B3Hash::digest(file)),
        None => (
            Verdict::Unasked {
                why: "no browser observation was given; the product does not run the page",
            },
            Vec::new(),
        ),
    };
    let held = opened.as_ref().ok();
    Report {
        digest: held.map(|(_, bundle)| bundle.digest()),
        events: held.map(|(_, bundle)| bundle.events()),
        structure: opened.as_ref().map_or_else(
            |found| Verdict::Failed {
                found: found.clone(),
            },
            |_| Verdict::Passed,
        ),
        bundle: against_bundle(held, asked.bundle),
        source: against_city(held, asked.city.as_ref()),
        offline,
        browser,
        covered,
    }
}

/// A bundle that reads, is consistent, and, inside a page, whose page's
/// references resolve against it; or what was found instead.
fn opened(bytes: &[u8], page: Option<&Page>) -> Result<(Document, Bundle), String> {
    let (document, bundle) = decode(bytes).map_err(|err| err.subject().to_owned())?;
    consistent(&document).map_err(|err| err.subject().to_owned())?;
    if let Some(page) = page {
        let seqs: BTreeSet<Decimal> = document
            .events
            .iter()
            .chain(&document.context)
            .map(|entry| entry.seq)
            .collect();
        page.references(&seqs)?;
    }
    Ok((document, bundle))
}

fn offline_verdict(findings: Vec<String>) -> Verdict {
    let mut findings = findings.into_iter();
    match (findings.next(), findings.len()) {
        (None, _) => Verdict::Passed,
        (Some(found), 0) => Verdict::Failed { found },
        (Some(first), more) => Verdict::Failed {
            found: format!("{first} (and {more} more)"),
        },
    }
}

fn against_bundle(held: Option<&(Document, Bundle)>, other: Option<&[u8]>) -> Verdict {
    let Some(other) = other else {
        return Verdict::Unasked {
            why: "no other bundle was given",
        };
    };
    let Some((ours, _)) = held else {
        return unreadable();
    };
    let theirs = match opened(other, None) {
        Ok((theirs, _)) => theirs,
        Err(found) => {
            return Verdict::Unable {
                why: format!("the other bundle does not read: {found}"),
            };
        }
    };
    match first_difference(ours, &theirs) {
        None => Verdict::Passed,
        Some(section) => Verdict::Failed {
            found: format!("differs from the other bundle first in `{section}`"),
        },
    }
}

fn against_city(held: Option<&(Document, Bundle)>, city: Option<&City<'_>>) -> Verdict {
    let Some(city) = city else {
        return Verdict::Unasked {
            why: "no city was given",
        };
    };
    let Some((document, _)) = held else {
        return unreadable();
    };
    match recomputed(document, city.root, city.reader.clone()) {
        Ok(Recomputed::Same) => Verdict::Passed,
        Ok(Recomputed::Differs(section)) => Verdict::Failed {
            found: format!("differs from its recomputation first in `{section}`"),
        },
        Ok(Recomputed::CannotReproduce(why)) => Verdict::Unable { why },
        Err(err) => Verdict::Unable {
            why: format!(
                "the city could not be read ({}: {}); {}",
                err.code(),
                err.subject(),
                err.recovery()
            ),
        },
    }
}

fn unreadable() -> Verdict {
    Verdict::Unable {
        why: "the file holds no bundle that reads, so there is nothing to compare".to_owned(),
    }
}

/// What recomputing a bundle from its city found.
enum Recomputed {
    Same,
    Differs(&'static str),
    /// This build, this reader or this city cannot recompute it, and the
    /// text says what can.
    CannotReproduce(String),
}

fn recomputed(document: &Document, root: &Path, reader: Reader) -> Result<Recomputed, AxError> {
    let source = &document.source;
    if source.rules != PROJECTION_RULES {
        return Ok(Recomputed::CannotReproduce(format!(
            "exported under projection rules {}, and this build projects under {PROJECTION_RULES}; \
             check it with a build of rules {}, or export it again",
            source.rules, source.rules
        )));
    }
    if source.reader != reader.name() {
        return Ok(Recomputed::CannotReproduce(
            "exported for another reader than this check reads as; check it as the reader \
             it was exported for, or export it again"
                .to_owned(),
        ));
    }
    let request = Request {
        selection: Selection::from_chosen(&source.selection)?,
        reader,
        cutoff: Cutoff::At(Seq::new(source.cutoff.seq.0)),
    };
    Ok(match project(root, &request)? {
        Projected::Whole(again) => match first_difference(document, &again) {
            None => Recomputed::Same,
            Some(section) => Recomputed::Differs(section),
        },
        Projected::EndsAt(reached) => Recomputed::CannotReproduce(format!(
            "the city's ledger ends at seq {}, before the cutoff {}",
            reached.map_or_else(|| "none".to_owned(), |seq| seq.value().to_string()),
            source.cutoff.seq.0
        )),
    })
}

/// The first section in which two documents differ, if any.
fn first_difference(ours: &Document, theirs: &Document) -> Option<&'static str> {
    let sections: [(&'static str, bool); 11] = [
        ("schema", ours.schema == theirs.schema),
        ("source", ours.source == theirs.source),
        ("events", ours.events == theirs.events),
        ("context", ours.context == theirs.context),
        ("unknown", ours.unknown == theirs.unknown),
        ("runs", ours.runs == theirs.runs),
        ("moments", ours.moments == theirs.moments),
        ("messages", ours.messages == theirs.messages),
        ("checkpoints", ours.checkpoints == theirs.checkpoints),
        ("costs", ours.costs == theirs.costs),
        ("withheld", ours.withheld == theirs.withheld),
    ];
    sections
        .into_iter()
        .find(|(_, same)| !same)
        .map(|(section, _)| section)
}
