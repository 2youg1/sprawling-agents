// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A playback page read the way a browser's parser builds it, the bundle
//! it carries, and the references it makes (`crates/accounting/spec/Playback/Check.lean` §8-13).
//!
//! The page goes through html5ever's tree builder, so an element inside
//! `<svg>`, a `<noscript>` or a `<template>` is the element a browser
//! would build, not what a scan of the source text would guess. The sink
//! below keeps a flat table of every node in the order the parser made
//! it; the checks read that table and never the source.

use std::collections::BTreeSet;

use html5ever::tendril::{StrTendril, TendrilSink};
use html5ever::{Attribute, ParseOpts, QualName, ns, parse_document};
use kernel::{AxCode, AxError};

use super::PAGE_MAX_BYTES;
use super::check::{Asked, Verdict, check};
use super::document::Decimal;
use super::encode::Bundle;
use sink::Builder;

mod sink;

/// The empty data block a template holds once, and [`embed`] fills.
pub const BUNDLE_BLOCK: &str = r#"<script type="application/json" id="playback-bundle"></script>"#;

/// The `id` of the data block.
const BUNDLE_ID: &str = "playback-bundle";

/// The page `template` makes once the bundle's bytes are in its data
/// block, checked for structure and static offline use.
///
/// # Errors
/// `E_INVALID_ARGS` for a template that is not UTF-8, that does not hold
/// [`BUNDLE_BLOCK`] exactly once, that would exceed [`PAGE_MAX_BYTES`],
/// or whose page fails either check; the subject is the first finding.
pub fn embed(template: &[u8], bundle: &Bundle) -> Result<Vec<u8>, AxError> {
    let refuse = |subject: String| {
        AxError::failure(AxCode::InvalidArgs, "embed a playback bundle", subject).with_recovery(
            "write the template as UTF-8 HTML with one empty playback-bundle block, a \
             Content-Security-Policy first in its head, and nothing it loads from outside \
             (crates/city/skills/playback/SKILL.md)",
        )
    };
    let text = std::str::from_utf8(template)
        .map_err(|err| refuse(format!("the template is not UTF-8: {err}")))?;
    let bytes = std::str::from_utf8(bundle.bytes())
        .map_err(|err| refuse(format!("the bundle is not UTF-8: {err}")))?;
    let blocks = text.matches(BUNDLE_BLOCK).count();
    if blocks != 1 {
        return Err(refuse(format!(
            "the template holds the empty bundle block {blocks} times, where it needs it once"
        )));
    }
    // The one place the opening tag meets the closing one; the bundle
    // goes between them.
    let Some((open, close)) = BUNDLE_BLOCK.split_once("></") else {
        return Err(refuse(
            "the empty bundle block has no closing tag".to_owned(),
        ));
    };
    let page = text.replacen(BUNDLE_BLOCK, &format!("{open}>{bytes}</{close}"), 1);
    if page.len() > PAGE_MAX_BYTES {
        return Err(refuse(format!(
            "{} bytes, over {PAGE_MAX_BYTES}",
            page.len()
        )));
    }
    let report = check(page.as_bytes(), &Asked::default());
    match (report.structure, report.offline) {
        (Verdict::Passed, Verdict::Passed) => Ok(page.into_bytes()),
        (Verdict::Failed { found }, _) | (_, Verdict::Failed { found }) => Err(refuse(found)),
        (Verdict::Unasked { why }, _) | (_, Verdict::Unasked { why }) => {
            Err(refuse(why.to_owned()))
        }
        (Verdict::Unable { why }, _) | (_, Verdict::Unable { why }) => Err(refuse(why)),
    }
}

/// A page, as the nodes its parser built.
pub(super) struct Page {
    nodes: Vec<Node>,
}

/// One node the parser built. Only elements are kept with their content;
/// the document, comments, instructions and template contents are places
/// a child can hang from.
enum Node {
    Element(Element),
    Other,
}

/// One element, with the text the parser appended directly into it.
pub(super) struct Element {
    name: QualName,
    attrs: Vec<Attribute>,
    text: String,
    parent: Option<usize>,
    contents: Option<usize>,
}

/// Which grammar an element belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Space {
    Html,
    Svg,
    MathMl,
    Other,
}

impl Page {
    /// Parses `text` as a browser does, with scripting on.
    pub(super) fn read(text: &str) -> Page {
        parse_document(Builder::new(), ParseOpts::default()).one(StrTendril::from(text))
    }

    /// Every element, in the order the parser built them.
    pub(super) fn elements(&self) -> impl Iterator<Item = &Element> {
        self.nodes.iter().filter_map(|node| match node {
            Node::Element(element) => Some(element),
            Node::Other => None,
        })
    }

    /// The element `element` was last appended to, if that is one.
    pub(super) fn parent_of(&self, element: &Element) -> Option<&Element> {
        match self.nodes.get(element.parent?)? {
            Node::Element(parent) => Some(parent),
            Node::Other => None,
        }
    }

    /// The text of the one data block, which must be an HTML `script` of
    /// type `application/json`.
    ///
    /// # Errors
    /// What was found instead: no block, two, or one of another shape.
    pub(super) fn bundle_text(&self) -> Result<&str, String> {
        let mut blocks = self
            .elements()
            .filter(|element| element.attr("id") == Some(BUNDLE_ID));
        let block = blocks
            .next()
            .ok_or_else(|| format!("no element carries the id `{BUNDLE_ID}`"))?;
        if blocks.next().is_some() {
            return Err(format!("two elements carry the id `{BUNDLE_ID}`"));
        }
        if !block.is_html("script") || block.attr("type") != Some("application/json") {
            return Err(format!(
                "the `{BUNDLE_ID}` element is not a script of type application/json"
            ));
        }
        Ok(&block.text)
    }

    /// Every id is used once, every fragment link names one, and every
    /// `data-seq` names a seq the bundle holds.
    ///
    /// # Errors
    /// The first reference that does not hold.
    pub(super) fn references(&self, seqs: &BTreeSet<Decimal>) -> Result<(), String> {
        let mut ids = BTreeSet::new();
        for id in self.elements().filter_map(|element| element.attr("id")) {
            if !ids.insert(id) {
                return Err(format!("the id `{id}` is used twice"));
            }
        }
        for element in self.elements() {
            let links = [element.attr("href"), element.xlink_href()];
            for target in links.into_iter().flatten() {
                if let Some(fragment) = target.strip_prefix('#')
                    && !fragment.is_empty()
                    && !ids.contains(fragment)
                {
                    return Err(format!("the link `{target}` names no id on the page"));
                }
            }
            if let Some(raw) = element.attr("data-seq") {
                let named = raw.parse::<u64>().ok().map(Decimal);
                if !named.is_some_and(|seq| seqs.contains(&seq)) {
                    return Err(format!(
                        "data-seq `{raw}` names no seq in the bundle's events or context"
                    ));
                }
            }
        }
        Ok(())
    }
}

impl Element {
    /// The element's local name.
    pub(super) fn local(&self) -> &str {
        &self.name.local
    }

    pub(super) fn space(&self) -> Space {
        if self.name.ns == ns!(html) {
            Space::Html
        } else if self.name.ns == ns!(svg) {
            Space::Svg
        } else if self.name.ns == ns!(mathml) {
            Space::MathMl
        } else {
            Space::Other
        }
    }

    /// Whether this is the HTML element `local`.
    pub(super) fn is_html(&self, local: &str) -> bool {
        self.space() == Space::Html && self.local() == local
    }

    /// The attribute `local` in no namespace, which is every attribute of
    /// an HTML element and the plain ones of a foreign element.
    pub(super) fn attr(&self, local: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|attr| attr.name.ns == ns!() && &*attr.name.local == local)
            .map(|attr| &*attr.value)
    }

    /// The xlink `href`, which a foreign element may carry beside `href`.
    pub(super) fn xlink_href(&self) -> Option<&str> {
        self.attrs
            .iter()
            .find(|attr| attr.name.ns == ns!(xlink) && &*attr.name.local == "href")
            .map(|attr| &*attr.value)
    }

    /// Every attribute as its namespace's word, its local name and value.
    pub(super) fn attributes(&self) -> impl Iterator<Item = (AttrSpace, &str, &str)> {
        self.attrs.iter().map(|attr| {
            let space = if attr.name.ns == ns!() {
                AttrSpace::Plain
            } else if attr.name.ns == ns!(xlink) {
                AttrSpace::Xlink
            } else {
                AttrSpace::Other
            };
            (space, &*attr.name.local, &*attr.value)
        })
    }

    /// The text the parser appended directly into this element.
    pub(super) fn text(&self) -> &str {
        &self.text
    }
}

/// The namespace an attribute sits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AttrSpace {
    Plain,
    Xlink,
    Other,
}
