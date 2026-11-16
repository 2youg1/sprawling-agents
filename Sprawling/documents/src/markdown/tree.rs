// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::encoding::Encoding;
use crate::span::Span;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Preview {
    Laid(Laid),

    Unsupported { encoding: Encoding },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Laid {
    pub span: Span,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ReplyState {
    Streaming,

    Settled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Block {
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

    Rule {
        span: Span,
    },

    Footnote {
        span: Span,
        name: String,
        blocks: Vec<Block>,
    },

    Unsupported {
        span: Span,
        construct: Construct,
        source: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ListItem {
    pub span: Span,
    pub check: Check,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Row {
    pub cells: Vec<Vec<Inline>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Inline {
    Text(String),
    Code(String),
    Emphasis(Vec<Inline>),
    Strong(Vec<Inline>),
    Strikethrough(Vec<Inline>),

    Link {
        target: String,
        title: String,
        content: Vec<Inline>,
    },

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

    Unsupported {
        construct: Construct,
        source: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Order {
    Bullet,
    Ordered { start: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Spacing {
    Tight,
    Loose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Check {
    NotATask,
    Open,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Align {
    None,
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Construct {
    Html,

    Math,

    FrontMatter,

    Nesting,

    Extension,
}
