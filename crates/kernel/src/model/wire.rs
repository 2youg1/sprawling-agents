// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The canonical conversation vocabulary: what a request and a response
//! are made of, in the city dialect every adapter translates from.

use std::num::NonZeroU64;

use serde::{Deserialize, Serialize};

use crate::budget::Tokens;
use crate::event::Payload;
use crate::model::image::ImageRef;
use crate::tool::ToolName;
/// Building-level constraints riding along the call. S2 carries the one
/// load-bearing bit; further fields only grow (14.3).
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildingPolicy {
    /// Confidential buildings lock the call to the local model pool;
    /// content never leaves the machine (11.4).
    pub confidential: bool,
}

impl BuildingPolicy {
    pub fn new(confidential: bool) -> Self {
        BuildingPolicy { confidential }
    }
}

/// Message author on the canonical (Anthropic-shaped) conversation.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
}

/// Why the provider stopped. Verdict-adjacent but wire-borne, so it stays
/// open like other wire enums (14.3).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    ToolUse,
    MaxTokens,
}

/// The most a model may emit in one response, thinking included.
///
/// **Zero is unrepresentable, and absence is not zero.** A provider
/// answers a zero ceiling with nothing at all, and a reply with nothing
/// in it reads downstream as work that finished — so a figure nobody
/// registered has to be carried as a figure nobody registered, and the
/// dialect that cannot write the request without one refuses the call
/// instead of inventing a number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Ceiling(NonZeroU64);

impl Ceiling {
    /// `None` for zero, which is not a ceiling.
    pub const fn new(tokens: u64) -> Option<Ceiling> {
        match NonZeroU64::new(tokens) {
            Some(tokens) => Some(Ceiling(tokens)),
            None => None,
        }
    }

    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl std::fmt::Display for Ceiling {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.get())
    }
}

/// One frozen-prefix block on the wire. `cache` marks an explicit prompt
/// cache breakpoint (breakpoints sit on segment edges).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemBlock {
    pub text: String,
    pub cache: bool,
}

/// Which wire the far side speaks. Open for growth; every match on it
/// handles the known kinds exhaustively and fails closed.
///
/// Lives here rather than in `gateway` for the same reason
/// [`BuildingPolicy`] does: two outer crates must name it (the gateway
/// translates it, the wire carries it) and neither may name the other.
/// `gateway::dialect` is its evaluator, not its definition.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DialectKind {
    Anthropic,
    OpenAi,
}

/// What a chosen model is for. Exhaustive rather than a free label: a
/// tag exists because some code asks for a model by it, so a tag with no
/// asker is a setting the person can fill in and never see used. It
/// grows when a caller appears, not when a name is imagined.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ModelTag {
    /// The model a resident thinks with.
    Main,
    /// The small model that reads long documents so the main one does
    /// not have to: summaries, structure trees, search results.
    Digest,
    /// The model that turns a recording into text.
    ///
    /// A tag rather than a store of its own, because "which endpoint and
    /// which model answer this kind of job" already has a mechanism: a
    /// person attaches an endpoint and chooses a model for a tag. A
    /// second form and a second file would be a second answer to one
    /// question, and the credential would need a second way into the
    /// vault.
    Transcribe,
}

impl ModelTag {
    /// Every tag, in the order a settings page should offer them.
    pub const ALL: [ModelTag; 3] = [ModelTag::Main, ModelTag::Digest, ModelTag::Transcribe];

    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelTag::Main => "main",
            ModelTag::Digest => "digest",
            ModelTag::Transcribe => "transcribe",
        }
    }
}

impl std::fmt::Display for ModelTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How hard the provider should think before answering, ordered from
/// least to most. Both dialects accept every level; they disagree only
/// on where `None` is written (an effort value on one wire, a separate
/// thinking field on the other).
///
/// The ladder mirrors the providers' own vocabularies; check theirs
/// before changing it:
/// <https://platform.claude.com/docs/en/build-with-claude/effort> and
/// <https://developers.openai.com/api/docs/guides/reasoning>.
///
/// Absence (`Option::None`) is not `Effort::None`: absence leaves the
/// choice to the provider, `Effort::None` asks it not to think.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Effort {
    None,
    Low,
    Medium,
    High,
    #[serde(rename = "xhigh")]
    XHigh,
    Max,
}

/// Canonical content block. Tool inputs are [`Payload`] — the float ban
/// holds here because these bytes become ledger payloads verbatim.
///
/// `Thinking` and `RedactedThinking` are carried unchanged end to end:
/// the provider verifies `signature` against the reasoning it issued, and
/// altering either block earns a 400 that names them as unmodifiable.
/// The block shapes are the city dialect's, so
/// <https://platform.claude.com/docs/en/build-with-claude/thinking> is
/// what a change to them has to agree with.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    Thinking {
        thinking: String,
        signature: String,
    },
    RedactedThinking {
        data: String,
    },
    ToolUse {
        id: String,
        name: ToolName,
        input: Payload,
    },
    /// A picture the model can see. Carries the reference and the
    /// dimensions; the bytes are fetched at the wire by whoever is about
    /// to send them (`gateway::endpoint`), so this block stays a thing a
    /// ledger can hold.
    Image(ImageRef),
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
        /// Pictures the tool produced. Absent from every tool result
        /// written before this field existed, which is why it reads as
        /// an empty list rather than as a broken record: an old history
        /// has to replay.
        #[serde(default)]
        attachments: Vec<ImageRef>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: Vec<ContentBlock>,
}

/// A tool as the provider sees it; sourced from catalog `tool_defs` only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: ToolName,
    pub description: String,
    pub input_schema: Payload,
}

/// The canonical request conversation (city dialect = Anthropic Messages). Both production dialects and the scripted model
/// consume this one shape — the seam carries it so replay, citysim and
/// the real gateway argue about the same object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    /// Absent when no catalogue row states this model's ceiling. The
    /// OpenAI wire then omits the field and takes the provider's own
    /// default; the Anthropic wire, which requires it, refuses.
    pub max_tokens: Option<Ceiling>,
    pub system: Vec<SystemBlock>,
    pub messages: Vec<ChatMessage>,
    pub tools: Vec<ToolDef>,
    /// Frozen at run start: changing it mid-run would start a new cached
    /// prompt prefix, so the value rides in from [`crate::FrozenConfig`].
    pub effort: Option<Effort>,
}

/// Provider-reported token counts; absent wire fields read as zero.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelUsage {
    pub input_tokens: Tokens,
    pub output_tokens: Tokens,
    pub cache_read_tokens: Tokens,
    pub cache_write_tokens: Tokens,
}

/// The canonical response: dialect output, endpoint input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: Vec<ContentBlock>,
    pub stop: StopReason,
    pub usage: ModelUsage,
}

impl ChatRequest {
    /// An empty conversation shell for adapters and tests that argue
    /// about hashes, not content.
    pub fn empty(model: &str, max_tokens: Ceiling) -> ChatRequest {
        ChatRequest {
            model: model.to_owned(),
            max_tokens: Some(max_tokens),
            system: Vec::new(),
            messages: Vec::new(),
            tools: Vec::new(),
            effort: None,
        }
    }
}
