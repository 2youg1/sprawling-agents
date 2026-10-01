// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! comrak's tree read into the preview tree (`crates/documents/Spec.lean`
//! D22-D24, D26).
//!
//! comrak owns CommonMark and GFM; this module owns which extensions the
//! grammar reads and how deep a container may nest, and asks `target`
//! which links become links and `position` where each block lies.
//! comrak's arena lives for one call and is dropped with it.

use comrak::nodes::{
    AstNode, ListType, NodeCodeBlock, NodeLink, NodeList, NodeValue, TableAlignment,
};
use comrak::{Arena, Options, parse_document};

use super::position::Positions;
use super::target::admitted;
use super::tree::{Align, Block, Check, Construct, Inline, ListItem, Order, Row, Spacing};
use crate::span::{Span, offset};

/// How many containers, blocks and inline marks together, a preview
/// nests before the rest is shown as source (D24): sixteen levels of the
/// deepest container on the wire stay well inside the 128 levels
/// `serde_json` reads by default.
pub(super) const NESTING_MAX: usize = 16;

/// The blocks of `text`, whose first byte is byte `at` of the version.
pub(super) fn blocks(text: &str, at: u64) -> Vec<Block> {
    let arena = Arena::new();
    let root = parse_document(&arena, text, &grammar());
    let lowering = Lowering {
        positions: Positions::of(text, at),
    };
    lowering.children(root, 0)
}

/// The one Markdown grammar (D22): CommonMark, GFM's tables,
/// strikethrough, autolinks and task lists, footnotes left where they
/// stand, dollar formulas, `---` front matter, and emphasis that closes
/// beside CJK punctuation. Nothing that rewrites the source.
fn grammar() -> Options<'static> {
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.footnotes = true;
    options.extension.math_dollars = true;
    options.extension.front_matter_delimiter = Some("---".to_owned());
    options.extension.cjk_friendly_emphasis = true;
    options.parse.leave_footnote_definitions = true;
    options
}

struct Lowering<'t> {
    positions: Positions<'t>,
}

impl Lowering<'_> {
    fn children<'a>(&self, parent: &'a AstNode<'a>, depth: usize) -> Vec<Block> {
        parent
            .children()
            .map(|child| self.block(child, depth))
            .collect()
    }

    fn block<'a>(&self, node: &'a AstNode<'a>, depth: usize) -> Block {
        let ast = node.data();
        let span = self.positions.span(ast.sourcepos);
        if depth >= NESTING_MAX {
            return self.unsupported(span, Construct::Nesting, "");
        }
        let inner = depth.saturating_add(1);
        match &ast.value {
            NodeValue::Heading(heading) => Block::Heading {
                span,
                level: heading.level,
                inline: self.inlines(node, inner),
            },
            NodeValue::Paragraph => Block::Paragraph {
                span,
                inline: self.inlines(node, inner),
            },
            NodeValue::List(list) => self.list(node, span, list, inner),
            NodeValue::BlockQuote => Block::Quote {
                span,
                blocks: self.children(node, inner),
            },
            NodeValue::CodeBlock(code) => code_block(span, code),
            NodeValue::Table(table) => Block::Table {
                span,
                align: table.alignments.iter().map(align).collect(),
                head: self
                    .rows(node, Head::Kept, inner)
                    .into_iter()
                    .next()
                    .unwrap_or(Row { cells: Vec::new() }),
                body: self.rows(node, Head::Skipped, inner),
            },
            NodeValue::ThematicBreak => Block::Rule { span },
            NodeValue::FootnoteDefinition(footnote) => Block::Footnote {
                span,
                name: footnote.name.clone(),
                blocks: self.children(node, inner),
            },
            NodeValue::HtmlBlock(html) => self.unsupported(span, Construct::Html, &html.literal),
            NodeValue::FrontMatter(front) => {
                self.unsupported(span, Construct::FrontMatter, front.trim_end())
            }
            NodeValue::Document
            | NodeValue::Item(_)
            | NodeValue::TaskItem(_)
            | NodeValue::TableRow(_)
            | NodeValue::TableCell
            | NodeValue::DescriptionList
            | NodeValue::DescriptionItem(_)
            | NodeValue::DescriptionTerm
            | NodeValue::DescriptionDetails
            | NodeValue::MultilineBlockQuote(_)
            | NodeValue::Alert(_)
            | NodeValue::Subtext
            | NodeValue::Text(_)
            | NodeValue::SoftBreak
            | NodeValue::LineBreak
            | NodeValue::Code(_)
            | NodeValue::HtmlInline(_)
            | NodeValue::Raw(_)
            | NodeValue::Emph
            | NodeValue::Strong
            | NodeValue::Strikethrough
            | NodeValue::Highlight
            | NodeValue::Superscript
            | NodeValue::Link(_)
            | NodeValue::Image(_)
            | NodeValue::FootnoteReference(_)
            | NodeValue::Math(_)
            | NodeValue::Escaped
            | NodeValue::WikiLink(_)
            | NodeValue::Underline
            | NodeValue::Subscript
            | NodeValue::SpoileredText
            | NodeValue::EscapedTag(_) => self.unsupported(span, Construct::Extension, ""),
        }
    }

    fn list<'a>(&self, node: &'a AstNode<'a>, span: Span, list: &NodeList, inner: usize) -> Block {
        let order = match list.list_type {
            ListType::Bullet => Order::Bullet,
            ListType::Ordered => Order::Ordered {
                start: offset(list.start),
            },
        };
        let spacing = if list.tight {
            Spacing::Tight
        } else {
            Spacing::Loose
        };
        let items = node
            .children()
            .map(|item| {
                let ast = item.data();
                let check = if let NodeValue::TaskItem(task) = &ast.value {
                    match task.symbol {
                        Some(_) => Check::Done,
                        None => Check::Open,
                    }
                } else {
                    Check::NotATask
                };
                ListItem {
                    span: self.positions.span(ast.sourcepos),
                    check,
                    blocks: self.children(item, inner),
                }
            })
            .collect();
        Block::List {
            span,
            order,
            spacing,
            items,
        }
    }

    /// The rows of a table, the head row alone or every other row.
    fn rows<'a>(&self, table: &'a AstNode<'a>, head: Head, inner: usize) -> Vec<Row> {
        table
            .children()
            .filter(|row| {
                let is_head = matches!(row.data().value, NodeValue::TableRow(true));
                match head {
                    Head::Kept => is_head,
                    Head::Skipped => !is_head,
                }
            })
            .map(|row| Row {
                cells: row
                    .children()
                    .map(|cell| self.inlines(cell, inner))
                    .collect(),
            })
            .collect()
    }

    fn inlines<'a>(&self, parent: &'a AstNode<'a>, depth: usize) -> Vec<Inline> {
        let mut out = Vec::new();
        for child in parent.children() {
            self.inline(child, depth, &mut out);
        }
        out
    }

    fn inline<'a>(&self, node: &'a AstNode<'a>, depth: usize, out: &mut Vec<Inline>) {
        let ast = node.data();
        if depth >= NESTING_MAX {
            out.push(Inline::Unsupported {
                construct: Construct::Nesting,
                source: plain(node),
            });
            return;
        }
        let inner = depth.saturating_add(1);
        match &ast.value {
            NodeValue::Text(text) => out.push(Inline::Text(text.to_string())),
            NodeValue::Code(code) => out.push(Inline::Code(code.literal.clone())),
            NodeValue::Emph => out.push(Inline::Emphasis(self.inlines(node, inner))),
            NodeValue::Strong => out.push(Inline::Strong(self.inlines(node, inner))),
            NodeValue::Strikethrough => out.push(Inline::Strikethrough(self.inlines(node, inner))),
            NodeValue::Link(link) => self.link(node, link, inner, out),
            NodeValue::Image(image) => out.push(image_of(node, image)),
            NodeValue::FootnoteReference(reference) => out.push(Inline::FootnoteReference {
                name: reference.name.clone(),
            }),
            NodeValue::SoftBreak => out.push(Inline::SoftBreak),
            NodeValue::LineBreak => out.push(Inline::LineBreak),
            NodeValue::HtmlInline(html) => out.push(Inline::Unsupported {
                construct: Construct::Html,
                source: html.clone(),
            }),
            NodeValue::Math(math) => out.push(Inline::Unsupported {
                construct: Construct::Math,
                source: self
                    .positions
                    .source(self.positions.span(ast.sourcepos))
                    .unwrap_or_else(|| math.literal.clone()),
            }),
            NodeValue::Raw(raw) => out.push(Inline::Text(raw.clone())),
            NodeValue::EscapedTag(tag) => out.push(Inline::Text((*tag).to_owned())),
            NodeValue::Escaped
            | NodeValue::Highlight
            | NodeValue::Superscript
            | NodeValue::Subscript
            | NodeValue::Underline
            | NodeValue::SpoileredText
            | NodeValue::WikiLink(_) => out.extend(self.inlines(node, inner)),
            NodeValue::Document
            | NodeValue::FrontMatter(_)
            | NodeValue::BlockQuote
            | NodeValue::List(_)
            | NodeValue::Item(_)
            | NodeValue::DescriptionList
            | NodeValue::DescriptionItem(_)
            | NodeValue::DescriptionTerm
            | NodeValue::DescriptionDetails
            | NodeValue::CodeBlock(_)
            | NodeValue::HtmlBlock(_)
            | NodeValue::Paragraph
            | NodeValue::Heading(_)
            | NodeValue::ThematicBreak
            | NodeValue::FootnoteDefinition(_)
            | NodeValue::Table(_)
            | NodeValue::TableRow(_)
            | NodeValue::TableCell
            | NodeValue::TaskItem(_)
            | NodeValue::MultilineBlockQuote(_)
            | NodeValue::Alert(_)
            | NodeValue::Subtext => out.push(Inline::Unsupported {
                construct: Construct::Extension,
                source: plain(node),
            }),
        }
    }

    /// A link the grammar admits, or its words as words.
    fn link<'a>(
        &self,
        node: &'a AstNode<'a>,
        link: &NodeLink,
        inner: usize,
        out: &mut Vec<Inline>,
    ) {
        let content = self.inlines(node, inner);
        if admitted(&link.url) {
            out.push(Inline::Link {
                target: link.url.clone(),
                title: link.title.clone(),
                content,
            });
        } else {
            out.extend(content);
        }
    }

    fn unsupported(&self, span: Span, construct: Construct, literal: &str) -> Block {
        Block::Unsupported {
            span,
            construct,
            source: self
                .positions
                .source(span)
                .unwrap_or_else(|| literal.to_owned()),
        }
    }
}

/// Which rows of a table [`Lowering::rows`] reads.
#[derive(Clone, Copy)]
enum Head {
    Kept,
    Skipped,
}

fn code_block(span: Span, code: &NodeCodeBlock) -> Block {
    Block::Code {
        span,
        info: code.info.clone(),
        text: code.literal.clone(),
    }
}

fn align(alignment: &TableAlignment) -> Align {
    match alignment {
        TableAlignment::None => Align::None,
        TableAlignment::Left => Align::Left,
        TableAlignment::Center => Align::Center,
        TableAlignment::Right => Align::Right,
    }
}

/// An image the grammar admits, or its alternative text as words.
fn image_of<'a>(node: &'a AstNode<'a>, image: &NodeLink) -> Inline {
    let alt = plain(node);
    if admitted(&image.url) {
        Inline::Image {
            target: image.url.clone(),
            title: image.title.clone(),
            alt,
        }
    } else {
        Inline::Text(alt)
    }
}

/// The words under `node`, read without recursion so a subtree of any
/// depth costs no stack.
fn plain<'a>(node: &'a AstNode<'a>) -> String {
    node.descendants()
        .filter_map(|descendant| {
            let ast = descendant.data();
            if let NodeValue::Text(text) = &ast.value {
                Some(text.to_string())
            } else if let NodeValue::Code(code) = &ast.value {
                Some(code.literal.clone())
            } else if let NodeValue::Math(math) = &ast.value {
                Some(math.literal.clone())
            } else if matches!(ast.value, NodeValue::SoftBreak | NodeValue::LineBreak) {
                Some(" ".to_owned())
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests;
