// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a preview carries: a Markdown window read as blocks and inline
//! marks (`crates/documents/Spec.lean` D21).
//!
//! Data, never HTML. A page draws each variant as an element of its own,
//! so no character a model or a resident wrote is handed to `innerHTML`.
//! What the grammar reads but a page does not draw is [`Block::Unsupported`]
//! or [`Inline::Unsupported`] with its source text, which is a different
//! shape from an empty block (D22).

use serde::{Deserialize, Serialize};

use crate::encoding::Encoding;
use crate::span::Span;

/// One window of a Markdown version, laid out, or the reason it was not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Preview {
    Laid(Laid),
    /// The version is text in an encoding whose bytes do not line up with
    /// the text read from them, so no block could name its bytes (D25).
    Unsupported {
        encoding: Encoding,
    },
}

/// A stretch of Markdown that was read, and its blocks: what a preview
/// and a reply both answer (D32). An empty stretch has none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Laid {
    pub span: Span,
    pub blocks: Vec<Block>,
}

/// Whether a model's reply is still arriving (D30).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ReplyState {
    /// More text may follow, so only the blocks before the closure point
    /// are read.
    Streaming,
    /// The call settled, so the whole text is read, as a version holding
    /// it would be previewed.
    Settled,
}

/// One block of a window, with the bytes of the version it was read from
/// (D26).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Block {
    /// `level` is 1 to 6.
    Heading {
        span: Span,
        level: u8,
        inline: Vec<Inline>,
    },
    Paragraph {
        span: Span,
        inline: Vec<Inline>,
    },
    List {
        span: Span,
        order: Order,
        spacing: Spacing,
        items: Vec<ListItem>,
    },
    Quote {
        span: Span,
        blocks: Vec<Block>,
    },
    /// A fenced or indented code block; `info` is the fence's info string,
    /// empty for an indented block, and `text` may be empty.
    Code {
        span: Span,
        info: String,
        text: String,
    },
    Table {
        span: Span,
        align: Vec<Align>,
        head: Row,
        body: Vec<Row>,
    },
    /// A thematic break.
    Rule {
        span: Span,
    },
    /// A footnote's definition, where the source put it.
    Footnote {
        span: Span,
        name: String,
        blocks: Vec<Block>,
    },
    /// Read but not drawn: the page shows `source`, the block's own text.
    Unsupported {
        span: Span,
        construct: Construct,
        source: String,
    },
}

/// One item of a list, with the blocks inside it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ListItem {
    pub span: Span,
    pub check: Check,
    pub blocks: Vec<Block>,
}

/// One row of a table: one run of inline marks per cell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Row {
    pub cells: Vec<Vec<Inline>>,
}

/// One inline mark. Inline marks carry no span: the readers of today
/// line a preview up with its source block by block (D26).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Inline {
    Text(String),
    Code(String),
    Emphasis(Vec<Inline>),
    Strong(Vec<Inline>),
    Strikethrough(Vec<Inline>),
    /// A target the grammar admits: relative, `http`, `https` or `mailto`
    /// (D23). `title` is empty when the source gave none.
    Link {
        target: String,
        title: String,
        content: Vec<Inline>,
    },
    /// An image whose target the grammar admits, with its alternative text.
    Image {
        target: String,
        title: String,
        alt: String,
    },
    FootnoteReference {
        name: String,
    },
    SoftBreak,
    LineBreak,
    /// Read but not drawn: the page shows `source`.
    Unsupported {
        construct: Construct,
        source: String,
    },
}

/// Whether a list counts its items, and from where.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Order {
    Bullet,
    Ordered { start: u64 },
}

/// Whether a list's items are paragraphs apart or one line apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Spacing {
    Tight,
    Loose,
}

/// Whether an item is a task, and whether it is done.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Check {
    NotATask,
    Open,
    Done,
}

/// How a table column is aligned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Align {
    None,
    Left,
    Center,
    Right,
}

/// What a page is shown the source of instead of a drawing (D22, D24).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Construct {
    /// Raw HTML, a block or a tag.
    Html,
    /// A formula between dollar signs.
    Math,
    /// The metadata between two `---` lines at the top of a document.
    FrontMatter,
    /// A container deeper than `NESTING_MAX` levels, with all it holds.
    Nesting,
    /// A node of a comrak extension this grammar does not turn on; it
    /// appears only if the grammar's options change.
    Extension,
}
