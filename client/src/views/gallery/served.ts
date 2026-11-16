// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Key } from "../../core/lang";
import type { ModelFact } from "../../core/probed";
import type {
  AxCode,
  ApprovalClass,
  ApprovalItem,
  EndpointsAnswer,
  FiguresLine,
  ModelFactsSummary,
} from "../../wire";
import { ApprovalId, Ceiling, Locator, ServerLabel, TimeMs, Window } from "../../wire";
import { UNTUNED } from "./configured";

function facts(id: string, context: number | null, ceiling: number | null): ModelFactsSummary {
  return {
    id,
    context_tokens: context === null ? null : Window.make(context),
    max_output_tokens: ceiling === null ? null : Ceiling.make(ceiling),
    input_modalities: [],
    input_price: null,
    output_price: null,
    thinking: { levels: [], on: "unknown", from: "unknown", words: [] },
    generation: {
      output_pointer: "/max_tokens",
      settings: {},
      temperature: { support: "unknown" },
      top_p: { support: "unknown" },
      top_k: { support: "unknown" },
    },
    canonical: id,
    listing: "listed",
  };
}

const FABLE = "anthropic/claude-fable-5.1";
const NUCLEUS = "openai/gpt-nucleus-6";
const MUSE = "meta/muse-spark-1.3-contributor";
const SORA = "openai/sora-2";

export interface ModelRow {
  readonly id: string;
  readonly context: string;
  readonly ceiling: string;
}

export const CHOSEN: ModelRow = { id: FABLE, context: "204800", ceiling: "64000" };

export const MODELS: readonly ModelRow[] = [
  { id: MUSE, context: "131072", ceiling: "8192" },
  CHOSEN,
  { id: NUCLEUS, context: "400000", ceiling: "" },
];

export const PROBED: readonly ModelFact[] = [
  {
    id: FABLE,
    contextTokens: 204_800,
    maxOutputTokens: 64_000,
    inputModalities: ["image", "text"],
    inputPrice: "0.000003",
    outputPrice: "0.000015",
  },
  {
    id: NUCLEUS,
    contextTokens: 400_000,
    maxOutputTokens: null,
    inputModalities: ["text"],
    inputPrice: null,
    outputPrice: null,
  },
  {
    id: SORA,
    contextTokens: null,
    maxOutputTokens: null,
    inputModalities: ["video"],
    inputPrice: null,
    outputPrice: null,
  },
  {
    id: MUSE,
    contextTokens: null,
    maxOutputTokens: null,
    inputModalities: [],
    inputPrice: null,
    outputPrice: null,
  },
];

export const TYPED = "house/hidden-1";

export const FIGURED: readonly FiguresLine[] = [
  {
    id: FABLE,
    input: "text_image",
    window: { tokens: Window.make(204_800), from: "upstream" },
    ceiling: { sent: { tokens: Ceiling.make(64_000), from: "upstream" } },
  },
  {
    id: NUCLEUS,
    input: "text",
    window: { tokens: Window.make(400_000), from: "upstream" },
    ceiling: "provider_default",
  },
  { id: MUSE, input: "text", window: null, ceiling: "provider_default" },
  {
    id: TYPED,
    input: "text",
    window: { tokens: Window.make(200_000), from: "preset" },
    ceiling: { sent: { tokens: Ceiling.make(8192), from: "registered" } },
  },
];

export const ENDPOINTS: EndpointsAnswer = {
  chosen: [{ endpoint: "example", model: CHOSEN.id, tag: "main" }],
  endpoints: [
    {
      base_url: "https://api.example.test/v1",
      connection_kind: "openai_compat",
      dialect: "open_ai",
      has_credential: true,
      label: "Example",
      local: false,
      models: [facts(FABLE, 204_800, 64_000), facts(NUCLEUS, 400_000, null)],
      name: "example",
      tuning: {
        ...UNTUNED,
        accounts: [
          { id: ServerLabel.make("main"), reference: "secret:providers/example.main" },
          { id: ServerLabel.make("spare"), reference: "secret:providers/example.spare" },
        ],
      },
      account_status: [
        { id: ServerLabel.make("main"), key: "stored" },
        { id: ServerLabel.make("spare"), key: "missing" },
      ],
    },
    {
      base_url: "http://127.0.0.1:11434/v1",
      connection_kind: "openai_compat",
      dialect: "open_ai",
      has_credential: false,
      label: "local",
      local: true,
      models: [facts("local/qwen3", null, null)],
      name: "local",
      tuning: UNTUNED,
      account_status: [],
    },
  ],
};

export interface NoticeLine {
  readonly code: AxCode;

  readonly action: string;
  readonly subject: string;
  readonly recovery: string;

  readonly at: string;

  readonly count: number;
}

export interface NoticeDay {
  readonly heading: Key;
  readonly rows: readonly NoticeLine[];
}

export const NOTICE_DAYS: readonly NoticeDay[] = [
  {
    heading: "notices_today",
    rows: [

      {
        code: "E_CONFIG_INVALID",
        action: "dispatch",
        subject: "the model changed from `fake-small` to `fake-chat`",
        at: "09:12",
        count: 2,

        recovery: "the room is frozen against this session; set main back in setup, or run /new",
      },

      {
        code: "E_BUSY",
        action: "open session",
        subject: "hall/mayor",
        at: "09:04",
        count: 1,

        recovery: "a run is working in this room; run /stop first, then /new",
      },

      {
        code: "E_TIMEOUT",
        action: "probe",
        subject: "api.example.test",
        at: "08:47",
        count: 3,

        recovery: "the provider did not answer in time; ask the link to try again",
      },

      {
        code: "E_PATH_NOT_FOUND",
        action: "read",
        subject: "city/lab/west/NOTES.md",
        at: "08:31",
        count: 1,

        recovery: "that path is not on the listing this building answers with",
      },

      {
        code: "E_TOOL_UNAVAILABLE",
        action: "call",
        subject: "browser",
        at: "08:02",
        count: 1,

        recovery: "no engine is installed for this tool; run the doctor's recipe first",
      },
    ],
  },
  {
    heading: "notices_yesterday",
    rows: [

      {
        code: "E_MODEL_UNCHOSEN",
        action: "choose the main model",
        subject: "lab/east",
        at: "18:22",
        count: 1,

        recovery: "attach a provider on the settings page and pick a model for this tag",
      },

      {
        code: "E_GATE_DENIED",
        action: "write",
        subject: "hall/.sprawling/rules.md",
        at: "17:50",
        count: 1,

        recovery: "a rule stopped this write; the rule names the alternative",
      },

      {
        code: "E_CREDENTIAL_MISSING",
        action: "probe",
        subject: "api.deepseek.com",
        at: "16:11",
        count: 2,

        recovery: "file a key for this endpoint in setup, then probe again",
      },

      {
        code: "E_INVALID_ARGS",
        action: "steer",
        subject: "run 0199c0de",
        at: "15:39",
        count: 1,

        recovery: "the city could not read this request; the wire shape is in the SPEC",
      },
    ],
  },
  {
    heading: "rec_earlier",
    rows: [

      {
        code: "E_BUDGET_EXHAUSTED",
        action: "dispatch",
        subject: "lab/west",
        at: "11:26",
        count: 1,

        recovery: "this run has spent all it may; raise the budget or start another run",
      },

      {
        code: "E_WORKTREE_BUSY",
        action: "checkout",
        subject: "lab/west",
        at: "10:58",
        count: 1,

        recovery: "another run holds this worktree; run /stop to let go of it",
      },

      {
        code: "E_LOOP_SUSPECTED",
        action: "dispatch",
        subject: "hall/mayor",
        at: "09:41",
        count: 4,

        recovery: "the watchdog suspected a loop and stopped the run; steer it elsewhere",
      },
    ],
  },
];

export const TOASTS: readonly NoticeLine[] = [

  {
    code: "E_WIRE_MISMATCH",
    action: "connect",
    subject: "sprawling 0.0.6",
    at: "03:35",
    count: 1,

    recovery: "the client and the city disagree on the wire; reload after the city updates",
  },

  {
    code: "E_TIMEOUT",
    action: "ask",
    subject: "hall/mayor",
    at: "03:31",
    count: 2,

    recovery: "the city did not answer in time; ask the link to try again",
  },

  {
    code: "E_PROVIDER",
    action: "dispatch",
    subject: "api.example.test",
    at: "03:29",
    count: 1,

    recovery: "the provider failed to answer; the city kept the words for another try",
  },
];

export const REFUSED_FIELD: NoticeLine = {
  code: "E_ENDPOINT_DIALECT_UNSUPPORTED",

  action: "attach",

  subject: "api_gateway.internal",
  at: "03:35",
  count: 1,

  recovery: "this endpoint speaks another dialect; pick the shape it answers in",
};

export interface Utterance {
  readonly speaker: "person" | "resident";
  readonly text: string;
}

export const ROUND: readonly Utterance[] = [
  {
    speaker: "person",
    text: "Ask the mayor what this city should keep, and file the answer where the registry can find it.",
  },
  {
    speaker: "resident",
    text: "The mayor kept three things: the transcript of the first day, the screenshot of the settings page under forced colours, and the release for 0.0.6. All three are filed, and the registry lists them newest first.",
  },
];

export const EARLIER_SEGMENT: readonly Utterance[] = [
  { speaker: "person", text: "What is on the shelves in this building?" },
  {
    speaker: "resident",
    text: "Four holdings across three shelves, and one name the room asks for that no shelf has.",
  },
  { speaker: "person", text: "Pin the engineering ones for the next run." },
];

export interface TurnLine {
  readonly id: string;
  readonly turn: string;
  readonly at: string;
}

export const TURNS: readonly TurnLine[] = [
  { id: "turn-1", turn: "1", at: "11:02" },
  { id: "turn-2", turn: "2", at: "11:19" },
  { id: "turn-3", turn: "3", at: "11:47" },
];

export interface CallLine {
  readonly id: string;

  readonly what: string;
  readonly at: string;
}

export const CALLS: readonly CallLine[] = [
  { id: "call-1", what: "read: city/hall/Handoff.md", at: "11:20" },
  { id: "call-2", what: "exec: just check", at: "11:24" },
  { id: "call-3", what: "edit: city/hall/PLAN.md", at: "11:31" },
  { id: "call-4", what: "exec: cargo nextest", at: "11:44" },
];

const RAISED_FROM = Date.now() - 10 * 60_000;

function question(
  id: string,
  actor: string,
  what: string,
  key: readonly [ApprovalClass, string],
  at: number,
  tainted: boolean,
): ApprovalItem {
  return {
    id: ApprovalId.make(id),
    actor,
    action_desc: what,
    artifact: Locator.make(
      "cas:b3-0000000000000000000000000000000000000000000000000000000000000000",
    ),
    cluster_key: { class: key[0], detail: key[1] },
    created: TimeMs.make(RAISED_FROM + at * 60_000),
    tainted,
  };
}

export const ONE_QUESTION: ApprovalItem = question(
  "ai_1",
  "lab/east",
  "exec: rm -rf target",
  ["question", "rm"],
  1,
  false,
);

export const WAITING: readonly ApprovalItem[] = [
  ONE_QUESTION,
  question("ai_2", "lab/west", "edit: the building's own rules", ["question", "rules"], 2, false),
  question("ai_3", "lab/west", "edit: the building's own rules", ["question", "rules"], 3, false),
  question(
    "ai_4",
    "hall/mayor",
    "browser: open a page somebody linked",
    ["question", "open"],
    4,
    true,
  ),
];
