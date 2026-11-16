// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::borrow::Cow;
use std::num::NonZeroU64;

use serde::{Deserialize, Serialize};

use crate::event::Payload;
use crate::model::image::ImageRef;
use crate::model::usage::ModelUsage;
use crate::tool::ToolName;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ModelRef {
    pub endpoint: String,
    pub model: String,
}

impl std::fmt::Display for ModelRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.endpoint, self.model)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildingPolicy {

    pub confidential: bool,
}

impl BuildingPolicy {
    pub fn new(confidential: bool) -> Self {
        BuildingPolicy { confidential }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    ToolUse,
    MaxTokens,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Ceiling(NonZeroU64);

impl Ceiling {

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemBlock {
    pub text: String,
    pub cache: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DialectKind {
    Anthropic,

    OpenAi,

    OpenAiResponses,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ModelTag {

    Main,

    Digest,

    Transcribe,

    Ocr,
}

impl ModelTag {

    pub const ALL: [ModelTag; 4] = [
        ModelTag::Main,
        ModelTag::Digest,
        ModelTag::Transcribe,
        ModelTag::Ocr,
    ];

    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelTag::Main => "main",
            ModelTag::Digest => "digest",
            ModelTag::Transcribe => "transcribe",
            ModelTag::Ocr => "ocr",
        }
    }
}

impl std::fmt::Display for ModelTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Effort {
    None,
    Minimal,
    Low,
    Medium,
    High,
    #[serde(rename = "xhigh")]
    XHigh,
    Max,
}

impl Effort {

    pub const ALL: [Effort; 7] = [
        Effort::None,
        Effort::Minimal,
        Effort::Low,
        Effort::Medium,
        Effort::High,
        Effort::XHigh,
        Effort::Max,
    ];

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Effort::None => "none",
            Effort::Minimal => "minimal",
            Effort::Low => "low",
            Effort::Medium => "medium",
            Effort::High => "high",
            Effort::XHigh => "xhigh",
            Effort::Max => "max",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Increment {

    Said(String),

    Thought(String),
}

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

    Image(ImageRef),
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,

        #[serde(default)]
        attachments: Vec<ImageRef>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: Vec<ContentBlock>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MessageBreakpoint {
    #[default]
    Unmarked,
    Tail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SharedPrefix {
    #[default]
    Unshared,
    Leading(std::num::NonZeroUsize),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ToolDef {
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub name: ToolName,
    pub description: String,
    pub input_schema: Payload,
}

impl ToolDef {

    pub fn pin_bytes(&self) -> Result<Vec<u8>, crate::AxError> {
        serde_json::to_vec(self).map_err(|err| {
            crate::AxError::failure(
                crate::AxCode::InvalidArgs,
                "pin a tool definition",
                self.name.as_str(),
            )
            .with_recovery(format!("make the input schema serialisable as JSON: {err}"))
        })
    }

    pub fn digest(&self) -> Result<crate::B3Hash, crate::AxError> {
        self.pin_bytes().map(|bytes| crate::B3Hash::digest(&bytes))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatRequest<'a> {
    pub model: String,

    pub max_tokens: Option<Ceiling>,
    pub system: Vec<SystemBlock>,
    pub messages: Cow<'a, [ChatMessage]>,
    pub tools: Cow<'a, [ToolDef]>,

    #[serde(skip)]
    pub breakpoint: MessageBreakpoint,

    #[serde(skip)]
    pub shared: SharedPrefix,

    pub effort: Option<Effort>,

    #[serde(default)]
    pub generation: crate::GenerationSettings,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: Vec<ContentBlock>,
    pub stop: StopReason,
    pub usage: ModelUsage,
}

impl ChatRequest<'_> {

    pub fn empty(model: &str, max_tokens: Ceiling) -> ChatRequest<'static> {
        ChatRequest {
            model: model.to_owned(),
            max_tokens: Some(max_tokens),
            system: Vec::new(),
            messages: Cow::Borrowed(&[]),
            tools: Cow::Borrowed(&[]),
            breakpoint: MessageBreakpoint::Unmarked,
            shared: SharedPrefix::Unshared,
            effort: None,
            generation: crate::GenerationSettings::default(),
        }
    }

    pub fn carries_breakpoint(&self, index: usize) -> bool {
        match self.breakpoint {
            MessageBreakpoint::Unmarked => false,
            MessageBreakpoint::Tail => index.checked_add(1) == Some(self.messages.len()),
        }
    }

    pub fn shared_system(&self) -> Option<&[SystemBlock]> {
        match self.shared {
            SharedPrefix::Unshared => None,
            SharedPrefix::Leading(count) => self.system.get(..count.get()),
        }
    }

    pub fn into_owned(self) -> ChatRequest<'static> {
        ChatRequest {
            model: self.model,
            max_tokens: self.max_tokens,
            system: self.system,
            messages: Cow::Owned(self.messages.into_owned()),
            tools: Cow::Owned(self.tools.into_owned()),
            breakpoint: self.breakpoint,
            shared: self.shared,
            effort: self.effort,
            generation: self.generation,
        }
    }
}
