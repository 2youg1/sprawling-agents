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

use super::{
    CeilingField, ChatSpelling, EffortField, Face, HostPreset, ModelPreset, ReasoningReturn,
};

/// The hosts this city knows without asking.
///
/// Short on purpose. A host belongs here when this city can cite what
/// it serves; everything else reaches the same facts through the
/// endpoint's own model list, which is the rung above this table.
pub const PRESETS: [HostPreset; 13] = [
    HostPreset {
        host: "api.anthropic.com",
        faces: &[Face {
            dialect: DialectKind::Anthropic,
            path: "/v1",
        }],
        models: ANTHROPIC_MODELS,
        chat: ChatSpelling::DOCUMENTED,
        session_header: None,
        source: "https://platform.claude.com/docs/en/api/messages",
    },
    HostPreset {
        host: "api.openai.com",
        // Chat completions and responses are both served here, and the
        // person's own toggle says which one this registration means.
        faces: &[
            Face {
                dialect: DialectKind::OpenAiResponses,
                path: "/v1",
            },
            Face {
                dialect: DialectKind::OpenAi,
                path: "/v1",
            },
        ],
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
        // Chat and responses both answer straight under the host, and
        // the Anthropic-compatible face under `/anthropic`, to which
        // the Anthropic client appends `/v1/messages`
        // (<https://api-docs.deepseek.com/>,
        // <https://api-docs.deepseek.com/guides/responses_api>).
        faces: &[
            Face {
                dialect: DialectKind::OpenAi,
                path: "/",
            },
            Face {
                dialect: DialectKind::OpenAiResponses,
                path: "/",
            },
            Face {
                dialect: DialectKind::Anthropic,
                path: "/anthropic/v1",
            },
        ],
        models: DEEPSEEK_MODELS,
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
        // Responses is the face the vendor recommends; chat completions
        // is still served as its legacy face.
        faces: &[
            Face {
                dialect: DialectKind::OpenAiResponses,
                path: "/v1",
            },
            Face {
                dialect: DialectKind::OpenAi,
                path: "/v1",
            },
        ],
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
        faces: &[Face {
            dialect: DialectKind::OpenAi,
            path: "/api/v1",
        }],
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
        faces: &[Face {
            dialect: DialectKind::OpenAi,
            path: "/v1beta/openai",
        }],
        models: &[],
        chat: ChatSpelling::DOCUMENTED,
        session_header: None,
        source: "https://ai.google.dev/gemini-api/docs/openai",
    },
    HostPreset {
        // Zhipu's mainland platform; `api.z.ai` below is the same API
        // served outside mainland China.
        host: "open.bigmodel.cn",
        faces: &[Face {
            dialect: DialectKind::OpenAi,
            path: "/api/paas/v4",
        }],
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
        faces: &[Face {
            dialect: DialectKind::OpenAi,
            path: "/api/paas/v4",
        }],
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
        // Each model answers on the face its row names: chat,
        // responses or messages.
        faces: &[
            Face {
                dialect: DialectKind::OpenAi,
                path: "/zen/v1",
            },
            Face {
                dialect: DialectKind::OpenAiResponses,
                path: "/zen/v1",
            },
            Face {
                dialect: DialectKind::Anthropic,
                path: "/zen/v1",
            },
        ],
        models: &[],
        chat: ChatSpelling::DOCUMENTED,
        session_header: Some("x-opencode-session"),
        source: "https://opencode.ai/docs/go/",
    },
    HostPreset {
        // The Kimi Code membership API, which a member calls with a key
        // from the Kimi Code console and which does not hang under
        // `/v1` at all. Appending `/v1` to this host answers 404,
        // which is the same defect `openrouter.ai` is listed for.
        host: "api.kimi.com",
        // Called through the OpenAI client library with this base URL
        // (`packages/kosong/src/kosong/chat_provider/openai_common.py`
        // constructs `AsyncOpenAI(base_url=...)`). The membership guide
        // prints the Anthropic-compatible base as `/coding/`, to which
        // the Anthropic client appends `/v1/messages`, so both faces
        // hang under `/coding/v1`
        // (<https://www.kimi.com/en/help/kimi-code/membership-guide>).
        faces: &[
            Face {
                dialect: DialectKind::OpenAi,
                path: "/coding/v1",
            },
            Face {
                dialect: DialectKind::Anthropic,
                path: "/coding/v1",
            },
        ],
        // Pending: the ceilings of the Kimi models are stated by this
        // endpoint's own model list, which needs a member's key to
        // read, and no vendor page reachable from this machine states
        // them.
        models: &[],
        chat: MOONSHOT_CHAT,
        session_header: None,
        source: "https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/auth/platforms.py",
    },
    HostPreset {
        host: "api.moonshot.cn",
        faces: &[Face {
            dialect: DialectKind::OpenAi,
            path: "/v1",
        }],
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
        faces: &[Face {
            dialect: DialectKind::OpenAi,
            path: "/v1",
        }],
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
        faces: &[Face {
            dialect: DialectKind::OpenAi,
            path: "/compatible-mode/v1",
        }],
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
/// membership API serves the same models, and `kimi-k2.7-code` keeps
/// every earlier `reasoning_content` whatever the request says.
const MOONSHOT_CHAT: ChatSpelling = ChatSpelling {
    ceiling: CeilingField::MaxCompletionTokens,
    effort: EffortField::ReasoningEffort,
    reasoning: ReasoningReturn::AsReasoningContent,
};

/// Anthropic's documented ceilings. The Messages API requires
/// `max_tokens` in every request, so a missing row here is a call that
/// cannot be written at all.
///
/// The first four rows are the lineup the overview page lists as
/// current, whose windows it prints as `1M` and `200K` and whose
/// ceilings as `128K` and `64K`; each is read at a thousand to the
/// `K`, which is never above the figure the vendor meant. The rows
/// after them are earlier families the same page listed before.
const ANTHROPIC_MODELS: &[ModelPreset] = &[
    ModelPreset {
        id_prefix: "claude-fable-5-1",
        context_tokens: 1_000_000,
        max_output_tokens: 128_000,
        input: InputKinds::TextImage,
        source: "https://platform.claude.com/docs/en/about-claude/models/overview",
    },
    ModelPreset {
        id_prefix: "claude-opus-5-5",
        context_tokens: 1_000_000,
        max_output_tokens: 128_000,
        input: InputKinds::TextImage,
        source: "https://platform.claude.com/docs/en/about-claude/models/overview",
    },
    ModelPreset {
        id_prefix: "claude-sonnet-5-5",
        context_tokens: 1_000_000,
        max_output_tokens: 128_000,
        input: InputKinds::TextImage,
        source: "https://platform.claude.com/docs/en/about-claude/models/overview",
    },
    ModelPreset {
        id_prefix: "claude-haiku-4-5",
        context_tokens: 200_000,
        max_output_tokens: 64_000,
        input: InputKinds::TextImage,
        source: "https://platform.claude.com/docs/en/about-claude/models/overview",
    },
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

/// DeepSeek's two families. The API reference states the ceiling
/// exactly - "between 1 and 384K (393216)" - and the models page states
/// a `1M` window for both, read at a thousand to the `K`
/// (<https://api-docs.deepseek.com/quick_start/pricing>). The retired
/// `deepseek-v4-flash` name is still accepted and served by the flash
/// model, so the `deepseek-v4` prefix answers for it too.
const DEEPSEEK_MODELS: &[ModelPreset] = &[
    ModelPreset {
        id_prefix: "deepseek-flash",
        context_tokens: 1_000_000,
        max_output_tokens: 393_216,
        input: InputKinds::TextImage,
        source: "https://api-docs.deepseek.com/api/create-chat-completion",
    },
    ModelPreset {
        id_prefix: "deepseek-v4",
        context_tokens: 1_000_000,
        max_output_tokens: 393_216,
        input: InputKinds::Text,
        source: "https://api-docs.deepseek.com/api/create-chat-completion",
    },
];
