// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The canonical conversation vocabulary: what a request and a response
//! are made of, in the city dialect every adapter translates from.

use serde::{Deserialize, Serialize};

use crate::budget::Tokens;
use crate::event::Payload;
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
pub enum ModelTag {
    /// The model a resident thinks with.
    Main,
    /// The small model that reads long documents so the main one does
    /// not have to: summaries, structure trees, search results.
    Digest,
}

impl ModelTag {
    /// Every tag, in the order a settings page should offer them.
    pub const ALL: [ModelTag; 2] = [ModelTag::Main, ModelTag::Digest];

    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelTag::Main => "main",
            ModelTag::Digest => "digest",
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
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
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
    pub max_tokens: u64,
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
    pub fn empty(model: &str, max_tokens: u64) -> ChatRequest {
        ChatRequest {
            model: model.to_owned(),
            max_tokens,
            system: Vec::new(),
            messages: Vec::new(),
            tools: Vec::new(),
            effort: None,
        }
    }
}
