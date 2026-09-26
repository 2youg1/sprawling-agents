// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The rows themselves: which host serves what, and where each
//! statement was read.
//!
//! Split from the lookups beside them because a row is data a person
//! checks against a vendor's page and a lookup is code a person reads
//! for its rule, and one file holding both is read twice for two
//! reasons.

use kernel::DialectKind;

use crate::market::InputKinds;

use super::{CeilingField, ChatSpelling, EffortField, HostPreset, ModelPreset, ReasoningReturn};

/// The hosts this city knows without asking.
///
/// Short on purpose. A host belongs here when this city can cite what
/// it serves; everything else reaches the same facts through the
/// endpoint's own model list, which is the rung above this table.
pub const PRESETS: [HostPreset; 13] = [
    HostPreset {
        host: "api.anthropic.com",
        base_path: "/v1",
        dialect: Some(DialectKind::Anthropic),
        models: ANTHROPIC_MODELS,
        chat: ChatSpelling::DOCUMENTED,
        session_header: None,
        source: "https://platform.claude.com/docs/en/api/messages",
    },
    HostPreset {
        host: "api.openai.com",
        base_path: "/v1",
        // Chat completions and responses are both served here, and the
        // person's own toggle says which one this registration means.
        dialect: None,
        models: OPENAI_MODELS,
        // `max_tokens` "is not compatible with o-series models"
        // (`CreateChatCompletionRequest` in `openai/openai-openapi`).
        chat: ChatSpelling {
            ceiling: CeilingField::MaxCompletionTokens,
            ..ChatSpelling::DOCUMENTED
        },
        session_header: None,
        source: "https://github.com/openai/openai-openapi/blob/master/openapi.yaml",
    },
    HostPreset {
        // The documented base URL carries no path: the chat face is
        // `/chat/completions` straight under the host.
        host: "api.deepseek.com",
        base_path: "/",
        // Chat and responses both answer here, messages under
        // `/anthropic`.
        dialect: None,
        models: &[],
        // Thinking is on by default, and a request with tools whose
        // history lacks the earlier `reasoning_content` answers 400.
        chat: ChatSpelling {
            reasoning: ReasoningReturn::AsReasoningContent,
            ..ChatSpelling::DOCUMENTED
        },
        session_header: None,
        source: "https://api-docs.deepseek.com/guides/thinking_mode",
    },
    HostPreset {
        host: "api.x.ai",
        base_path: "/v1",
        // Chat completions and responses are both served here.
        dialect: None,
        models: &[],
        // `max_tokens` is deprecated in favour of
        // `max_completion_tokens`.
        chat: ChatSpelling {
            ceiling: CeilingField::MaxCompletionTokens,
            ..ChatSpelling::DOCUMENTED
        },
        session_header: None,
        source: "https://docs.x.ai/developers/rest-api-reference/inference/chat-completions",
    },
    HostPreset {
        host: "openrouter.ai",
        // Not `/v1`: this aggregator serves under `/api/v1`, and the
        // rule that appends `/v1` to every path-less URL answers 404
        // here.
        base_path: "/api/v1",
        dialect: Some(DialectKind::OpenAi),
        // This gateway states window, ceiling, modalities and prices in
        // its own model list, so a row here would be a second home for
        // a fact the endpoint already publishes.
        models: &[],
        // Its own `reasoning` object carries `max`, which its
        // `reasoning_effort` enum does not, and it takes an earlier
        // turn's reasoning back as `reasoning_content`.
        chat: ChatSpelling {
            ceiling: CeilingField::MaxTokens,
            effort: EffortField::ReasoningObject,
            reasoning: ReasoningReturn::AsReasoningContent,
        },
        session_header: None,
        source: "https://openrouter.ai/docs/guides/best-practices/reasoning-tokens",
    },
    HostPreset {
        host: "generativelanguage.googleapis.com",
        // The OpenAI-compatible face. `/v1beta` alone is Gemini's own
        // shape, which this city does not write.
        base_path: "/v1beta/openai",
        dialect: Some(DialectKind::OpenAi),
        models: &[],
        chat: ChatSpelling::DOCUMENTED,
        session_header: None,
        source: "https://ai.google.dev/gemini-api/docs/openai",
    },
    HostPreset {
        // Zhipu's mainland platform; `api.z.ai` below is the same API
        // served outside mainland China.
        host: "open.bigmodel.cn",
        base_path: "/api/paas/v4",
        dialect: Some(DialectKind::OpenAi),
        models: &[],
        // The assistant message takes `reasoning_content`, which
        // `clear_thinking: false` keeps in context.
        chat: ChatSpelling {
            reasoning: ReasoningReturn::AsReasoningContent,
            ..ChatSpelling::DOCUMENTED
        },
        session_header: None,
        source: "https://docs.bigmodel.cn/api-reference/%E6%A8%A1%E5%9E%8B-api/%E5%AF%B9%E8%AF%9D%E8%A1%A5%E5%85%A8",
    },
    HostPreset {
        host: "api.z.ai",
        base_path: "/api/paas/v4",
        dialect: Some(DialectKind::OpenAi),
        models: &[],
        chat: ChatSpelling {
            reasoning: ReasoningReturn::AsReasoningContent,
            ..ChatSpelling::DOCUMENTED
        },
        session_header: None,
        source: "https://docs.z.ai/api-reference/llm/chat-completion",
    },
    HostPreset {
        // OpenCode Zen. OpenCode Go hangs under `/zen/go/v1` on the
        // same host, so a Go registration is entered with its path.
        host: "opencode.ai",
        base_path: "/zen/v1",
        // Each model answers on the face its row names: chat,
        // responses or messages.
        dialect: None,
        models: &[],
        chat: ChatSpelling::DOCUMENTED,
        session_header: Some("x-opencode-session"),
        source: "https://opencode.ai/docs/go/",
    },
    HostPreset {
        // The Kimi Code subscription, whose API does not hang under
        // `/v1` at all. Appending `/v1` to this host answers 404,
        // which is the same defect `openrouter.ai` is listed for.
        host: "api.kimi.com",
        base_path: "/coding/v1",
        // Called through the OpenAI client library with this base URL
        // (`packages/kosong/src/kosong/chat_provider/openai_common.py`
        // constructs `AsyncOpenAI(base_url=...)`), so the chat face is
        // the OpenAI-compatible one.
        dialect: Some(DialectKind::OpenAi),
        // Pending: the ceilings of the Kimi models are stated by this
        // endpoint's own model list, which needs a subscription token
        // to read, and no vendor page reachable from this machine
        // states them.
        models: &[],
        chat: MOONSHOT_CHAT,
        session_header: None,
        source: "https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/auth/platforms.py",
    },
    HostPreset {
        host: "api.moonshot.cn",
        base_path: "/v1",
        dialect: Some(DialectKind::OpenAi),
        // Pending: as above, the model list is the statement and it
        // needs a key.
        models: &[],
        chat: MOONSHOT_CHAT,
        session_header: None,
        source: "https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/auth/platforms.py",
    },
    HostPreset {
        // The same platform served outside mainland China; one row per
        // host because a host is what a person pastes.
        host: "api.moonshot.ai",
        base_path: "/v1",
        dialect: Some(DialectKind::OpenAi),
        models: &[],
        chat: MOONSHOT_CHAT,
        session_header: None,
        source: "https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/auth/platforms.py",
    },
    HostPreset {
        // Dashscope's compatible mode on its one fixed host; the other
        // regions give each workspace a host of its own, entered with
        // its path.
        host: "dashscope-us.aliyuncs.com",
        base_path: "/compatible-mode/v1",
        dialect: Some(DialectKind::OpenAi),
        models: &[],
        chat: ChatSpelling::DOCUMENTED,
        session_header: None,
        source: "https://help.aliyun.com/zh/model-studio/compatibility-of-openai-with-dashscope",
    },
];

/// Moonshot's chat face: `max_tokens` is deprecated in favour of
/// `max_completion_tokens`, and every earlier assistant message keeps
/// its `reasoning_content`, read at
/// <https://platform.moonshot.ai/docs/api/chat>. The Kimi Code
/// subscription serves the same models, and `kimi-k2.7-code` keeps
/// every earlier `reasoning_content` whatever the request says.
const MOONSHOT_CHAT: ChatSpelling = ChatSpelling {
    ceiling: CeilingField::MaxCompletionTokens,
    effort: EffortField::ReasoningEffort,
    reasoning: ReasoningReturn::AsReasoningContent,
};

/// Anthropic's documented ceilings. The Messages API requires
/// `max_tokens` in every request, so a missing row here is a call that
/// cannot be written at all.
const ANTHROPIC_MODELS: &[ModelPreset] = &[
    ModelPreset {
        id_prefix: "claude-opus-4",
        context_tokens: 200_000,
        max_output_tokens: 32_000,
        input: InputKinds::TextImage,
        source: "https://platform.claude.com/docs/en/about-claude/models/overview",
    },
    ModelPreset {
        id_prefix: "claude-sonnet-4",
        context_tokens: 200_000,
        max_output_tokens: 64_000,
        input: InputKinds::TextImage,
        source: "https://platform.claude.com/docs/en/about-claude/models/overview",
    },
    ModelPreset {
        id_prefix: "claude-3-7-sonnet",
        context_tokens: 200_000,
        max_output_tokens: 64_000,
        input: InputKinds::TextImage,
        source: "https://platform.claude.com/docs/en/about-claude/models/overview",
    },
    ModelPreset {
        id_prefix: "claude-3-5-haiku",
        context_tokens: 200_000,
        max_output_tokens: 8_192,
        input: InputKinds::TextImage,
        source: "https://platform.claude.com/docs/en/about-claude/models/overview",
    },
];

/// OpenAI's documented ceilings.
const OPENAI_MODELS: &[ModelPreset] = &[
    ModelPreset {
        id_prefix: "gpt-4.1",
        context_tokens: 1_047_576,
        max_output_tokens: 32_768,
        input: InputKinds::TextImage,
        source: "https://platform.openai.com/docs/models/gpt-4.1",
    },
    ModelPreset {
        id_prefix: "gpt-4o",
        context_tokens: 128_000,
        max_output_tokens: 16_384,
        input: InputKinds::TextImage,
        source: "https://platform.openai.com/docs/models/gpt-4o",
    },
    ModelPreset {
        id_prefix: "o3",
        context_tokens: 200_000,
        max_output_tokens: 100_000,
        input: InputKinds::TextImage,
        source: "https://platform.openai.com/docs/models/o3",
    },
    ModelPreset {
        id_prefix: "o4-mini",
        context_tokens: 200_000,
        max_output_tokens: 100_000,
        input: InputKinds::TextImage,
        source: "https://platform.openai.com/docs/models/o4-mini",
    },
];
