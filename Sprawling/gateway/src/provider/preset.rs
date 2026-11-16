// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod rows;

pub use rows::PRESETS;

use kernel::{Ceiling, DialectKind};

use crate::market::InputKinds;

pub struct HostPreset {
    pub host: &'static str,

    pub faces: &'static [Face],

    pub models: &'static [ModelPreset],

    pub chat: ChatSpelling,

    pub session_header: Option<&'static str>,

    pub cache_key_field: Option<&'static str>,

    pub generation: GenerationPreset,

    pub source: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct GenerationPreset {
    pub anthropic: kernel::GenerationSupport,
    pub open_ai: kernel::GenerationSupport,
    pub responses: kernel::GenerationSupport,
}

impl GenerationPreset {
    pub const UNKNOWN: GenerationPreset = GenerationPreset {
        anthropic: kernel::GenerationSupport {
            temperature: None,
            top_p: None,
            top_k: None,
        },
        open_ai: kernel::GenerationSupport {
            temperature: None,
            top_p: None,
            top_k: None,
        },
        responses: kernel::GenerationSupport {
            temperature: None,
            top_p: None,
            top_k: None,
        },
    };
}

pub(crate) const ANTHROPIC_GENERATION: kernel::GenerationSupport = kernel::GenerationSupport {
    temperature: Some(kernel::ParameterSupport::Supported),
    top_p: Some(kernel::ParameterSupport::Supported),
    top_k: Some(kernel::ParameterSupport::Supported),
};

pub(crate) const OPENAI_GENERATION: kernel::GenerationSupport = kernel::GenerationSupport {
    temperature: None,
    top_p: None,
    top_k: Some(kernel::ParameterSupport::Unsupported),
};

pub struct Face {
    pub dialect: DialectKind,
    pub path: &'static str,
}

impl HostPreset {
    #[must_use]
    pub fn default_dialect(&self) -> Option<DialectKind> {
        match self.faces {
            [only] => Some(only.dialect),
            _ => None,
        }
    }

    #[must_use]
    pub fn path_for(&self, dialect: Option<DialectKind>) -> Option<&'static str> {
        dialect
            .and_then(|chosen| self.faces.iter().find(|face| face.dialect == chosen))
            .or_else(|| self.faces.first())
            .map(|face| face.path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownHost {
    pub host: &'static str,
    pub faces: Vec<(DialectKind, String)>,
}

pub fn known_hosts() -> Result<Vec<KnownHost>, kernel::AxError> {
    PRESETS
        .iter()
        .map(|row| {
            let faces = row
                .faces
                .iter()
                .map(|face| {
                    let stored = crate::router::normalise_entered(
                        row.host,
                        crate::router::DialectHint::of(face.dialect),
                    )?;
                    Ok((face.dialect, stored.base_url))
                })
                .collect::<Result<Vec<_>, kernel::AxError>>()?;
            Ok(KnownHost {
                host: row.host,
                faces,
            })
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChatSpelling {
    pub ceiling: CeilingField,
    pub effort: EffortField,
    pub reasoning: ReasoningReturn,
    pub chat_tool_pictures: ToolPictureCarry,
    pub responses_tool_pictures: ToolPictureCarry,
}

impl ChatSpelling {
    pub const DOCUMENTED: ChatSpelling = ChatSpelling {
        ceiling: CeilingField::MaxTokens,
        effort: EffortField::ReasoningEffort,
        reasoning: ReasoningReturn::Dropped,
        chat_tool_pictures: ToolPictureCarry::FollowingUser,
        responses_tool_pictures: ToolPictureCarry::FollowingUser,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolPictureCarry {
    FollowingUser,

    InToolContent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CeilingField {
    MaxTokens,

    MaxCompletionTokens,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffortField {
    ReasoningEffort,

    ReasoningObject,

    ThinkingToggle,

    EnableThinking,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningReturn {
    Dropped,

    AsReasoningContent,
}

pub struct ModelPreset {
    pub id_prefix: &'static str,
    pub context_tokens: u64,

    pub max_output_tokens: u64,
    pub input: InputKinds,

    pub image_max_side: Option<std::num::NonZeroU32>,

    pub thinking: Option<PresetThinking>,

    pub cache_min_tokens: Option<u64>,

    pub source: &'static str,
}

pub struct PresetThinking {
    pub levels: &'static [kernel::Effort],

    pub default: Option<kernel::Effort>,

    pub source: &'static str,
}

#[must_use]
pub fn for_host(host: &str) -> Option<&'static HostPreset> {
    PRESETS.iter().find(|row| row.host == host)
}

#[must_use]
pub fn chat_spelling(base_url: &str) -> ChatSpelling {
    host_row(base_url).map_or(ChatSpelling::DOCUMENTED, |row| row.chat)
}

#[must_use]
pub fn session_header(base_url: &str) -> Option<&'static str> {
    host_row(base_url)?.session_header
}

#[must_use]
pub(crate) fn cache_key_field(base_url: &str) -> Option<&'static str> {
    host_row(base_url)?.cache_key_field
}

fn host_row(base_url: &str) -> Option<&'static HostPreset> {
    let (host, _, _) = crate::reach::split(base_url)?;
    for_host(&host)
}

#[must_use]
pub fn model_for(base_url: &str, id: &str) -> Option<&'static ModelPreset> {
    row_for(
        host_row(base_url).map_or(&[][..], |row| row.models),
        PRESETS.iter().flat_map(|row| row.models.iter()),
        crate::reach::is_local(base_url),
        id,
    )
}

fn row_for<'rows>(
    own: &'rows [ModelPreset],
    vendors: impl Iterator<Item = &'rows ModelPreset>,
    local: bool,
    id: &str,
) -> Option<&'rows ModelPreset> {
    let relayed = own.is_empty() && !local;
    own.iter()
        .chain(vendors.filter(|_| relayed))
        .filter(|row| id.starts_with(row.id_prefix))
        .max_by_key(|row| row.id_prefix.len())
}

pub(crate) const CACHE_MIN_TOKENS_DEFAULT: u64 = 4_096;

#[must_use]
pub(crate) fn cache_min_tokens_for(base_url: &str, id: &str) -> u64 {
    model_for(base_url, id)
        .and_then(|row| row.cache_min_tokens)
        .unwrap_or(CACHE_MIN_TOKENS_DEFAULT)
}

#[must_use]
pub fn ceiling_for(base_url: &str, id: &str) -> Option<Ceiling> {
    Ceiling::new(model_for(base_url, id)?.max_output_tokens)
}

#[must_use]
pub fn window_for(base_url: &str, id: &str) -> Option<kernel::Window> {
    kernel::Window::new(model_for(base_url, id)?.context_tokens)
}

#[must_use]
pub(crate) fn input_for(base_url: &str, id: &str) -> Option<InputKinds> {
    model_for(base_url, id).map(|row| row.input)
}

#[must_use]
pub(crate) fn image_max_side(
    base_url: &str,
    id: &str,
    picture_parts: usize,
) -> Option<std::num::NonZeroU32> {
    let row = model_for(base_url, id)?;
    let ceiling = row.image_max_side?;
    if row.id_prefix.starts_with("deepseek-") && picture_parts >= 15 {
        std::num::NonZeroU32::new(4096).map(|limit| ceiling.min(limit))
    } else {
        Some(ceiling)
    }
}

#[must_use]
pub(crate) fn thinking_for(base_url: &str, id: &str) -> Option<&'static PresetThinking> {
    model_for(base_url, id)?.thinking.as_ref()
}

pub(crate) fn generation_for(base_url: &str, dialect: DialectKind) -> kernel::GenerationSupport {
    let Some(row) = host_row(base_url) else {
        return kernel::GenerationSupport::default();
    };
    match dialect {
        DialectKind::Anthropic => row.generation.anthropic,
        DialectKind::OpenAi => row.generation.open_ai,
        DialectKind::OpenAiResponses => row.generation.responses,
    }
}

pub struct Presets;

impl crate::router::HostDefaults for Presets {
    fn default_path(&self, host: &str, face: crate::router::DialectHint) -> Option<&str> {
        for_host(host)?.path_for(face.dialect())
    }

    fn default_dialect(&self, host: &str) -> Option<crate::router::DialectHint> {
        Some(crate::router::DialectHint::of(
            for_host(host)?.default_dialect()?,
        ))
    }
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
