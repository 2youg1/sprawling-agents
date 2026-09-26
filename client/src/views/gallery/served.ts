// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The answers a made-up city hands the fixtures, written once here.
//
// The provider section is read by three fixtures - the table on the
// settings page, the model column of the selector over the composer,
// and the shared controls that sort and correct a table - so the rows
// are written once here. A second spelling of the same three model ids
// would be three facts with two homes, and the first time somebody
// corrected one of them the two screens would disagree about what this
// provider serves. The notices and the talk rows below carry one
// refusal and one conversation past several drawings of it for the
// same reason.

import type { Key } from "../../core/lang";
import type { ModelFact } from "../../core/probed";
import type { AxCode, ApprovalClass, ApprovalItem, EndpointsAnswer, ModelFactsSummary } from "../../wire";
import { ApprovalId, Ceiling, Locator, TimeMs, Window } from "../../wire";

// The three model ids, each written once. Every list below names them
// through these, so a fixture cannot describe a model the endpoint does
// not serve.
// One catalogue row as a probe that read the provider's own list would
// have left it: the id, and whatever that list stated beside it.
function facts(id: string, context: number | null, ceiling: number | null): ModelFactsSummary {
  return {
    id,
    context_tokens: context === null ? null : Window.make(context),
    max_output_tokens: ceiling === null ? null : Ceiling.make(ceiling),
    input_modalities: [],
    input_price: null,
    output_price: null,
  };
}

const FABLE = "anthropic/claude-fable-5.1";
const NUCLEUS = "openai/gpt-nucleus-6";
const MUSE = "meta/muse-spark-1.3-contributor";
const SORA = "openai/sora-2";

// One row of the model table a provider's probe answers with, in the
// shape §3.4 asks for: an id, what it can read, and what it may write -
// the last of which a person corrects when the probe could not read it.
export interface ModelRow {
  readonly id: string;
  readonly context: string;
  readonly ceiling: string;
}

// The row every fixture treats as the chosen one, named rather than
// reached by its position: a list somebody reorders must not silently
// change which model a screen reports as picked.
export const CHOSEN: ModelRow = { id: FABLE, context: "204800", ceiling: "64000" };

export const MODELS: readonly ModelRow[] = [
  { id: MUSE, context: "131072", ceiling: "8192" },
  CHOSEN,
  { id: NUCLEUS, context: "400000", ceiling: "" },
];

// What one provider's probe answered, as a person meets it: three text
// models across three vendors, and one the text-only switch hides.
// Two rows state their own ceilings and prices, one states nothing but a
// name, and one is marked as video by the provider rather than by its
// spelling - which is the whole range this table has to render.
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

// Two attached providers, one with a key filed for it and one without,
// which is the difference the endpoint list exists to show.
export const ENDPOINTS: EndpointsAnswer = {
  chosen: [{ endpoint: "zenmux", model: CHOSEN.id, tag: "main" }],
  endpoints: [
    {
      base_url: "https://api.zenmux.ai/v1",
      connection_kind: "openai_compat",
      dialect: "open_ai",
      has_credential: true,
      label: "ZenMux",
      local: false,
      models: [facts(FABLE, 204_800, 64_000), facts(NUCLEUS, 400_000, null)],
      name: "zenmux",
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
    },
  ],
};

// One refusal as the notices fixtures draw it: the three sentences the
// city wrote, when it arrived, and how many times the same one did.
// `code` is typed as the wire's closed set, because the recovery verbs
// a notice offers come from the one table that maps codes to them.
export interface NoticeLine {
  readonly code: AxCode;
  // As the city wrote them: what failed, what it was against, and what
  // the person can do next. Never translated, never reworded here.
  readonly action: string;
  readonly subject: string;
  readonly recovery: string;
  // A clock string, already formatted by the caller's own clock.
  readonly at: string;
  // One is a single refusal; past one the notice shows the count.
  readonly count: number;
}

// A day of refusals under one heading. The heading is a `lang.json`
// key, so the day is named in the reader's language: today, yesterday,
// and the day whose only name is that it is earlier than those.
export interface NoticeDay {
  readonly heading: Key;
  readonly rows: readonly NoticeLine[];
}

// Twelve refusals across three days, newest day first. The subjects
// span three rooms and two runs, so the rows have something to merge
// on - a code that repeats under one subject - and something not to.
// The frozen-model refusal carries the sentence the runtime writes as
// its subject (`crates/runtime/src/turn/report.rs`), because a subject
// that is not a room is the case the notice's ways out must survive.
export const NOTICE_DAYS: readonly NoticeDay[] = [
  {
    heading: "notices_today",
    rows: [
      // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
      { code: "E_CONFIG_INVALID", action: "dispatch", subject: "the model changed from `fake-small` to `fake-chat`", at: "09:12", count: 2,
        // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
        recovery: "the room is frozen against this session; set main back in setup, or run /new" },
      // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
      { code: "E_BUSY", action: "open session", subject: "hall/mayor", at: "09:04", count: 1,
        // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
        recovery: "a run is working in this room; run /stop first, then /new" },
      // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
      { code: "E_TIMEOUT", action: "probe", subject: "api.zenmux.ai", at: "08:47", count: 3,
        // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
        recovery: "the provider did not answer in time; ask the link to try again" },
      // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
      { code: "E_PATH_NOT_FOUND", action: "read", subject: "city/lab/west/NOTES.md", at: "08:31", count: 1,
        // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
        recovery: "that path is not on the listing this building answers with" },
      // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
      { code: "E_TOOL_UNAVAILABLE", action: "call", subject: "browser", at: "08:02", count: 1,
        // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
        recovery: "no engine is installed for this tool; run the doctor's recipe first" },
    ],
  },
  {
    heading: "notices_yesterday",
    rows: [
      // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
      { code: "E_MODEL_UNCHOSEN", action: "choose the main model", subject: "lab/east", at: "18:22", count: 1,
        // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
        recovery: "attach a provider on the settings page and pick a model for this tag" },
      // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
      { code: "E_GATE_DENIED", action: "write", subject: "hall/.sprawling/rules.md", at: "17:50", count: 1,
        // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
        recovery: "a rule stopped this write; the rule names the alternative" },
      // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
      { code: "E_CREDENTIAL_MISSING", action: "probe", subject: "api.deepseek.com", at: "16:11", count: 2,
        // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
        recovery: "file a key for this endpoint in setup, then probe again" },
      // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
      { code: "E_INVALID_ARGS", action: "steer", subject: "run 0199c0de", at: "15:39", count: 1,
        // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
        recovery: "the city could not read this request; the wire shape is in the SPEC" },
    ],
  },
  {
    heading: "rec_earlier",
    rows: [
      // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
      { code: "E_BUDGET_EXHAUSTED", action: "dispatch", subject: "lab/west", at: "11:26", count: 1,
        // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
        recovery: "this run has spent all it may; raise the budget or start another run" },
      // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
      { code: "E_WORKTREE_BUSY", action: "checkout", subject: "lab/west", at: "10:58", count: 1,
        // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
        recovery: "another run holds this worktree; run /stop to let go of it" },
      // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
      { code: "E_LOOP_SUSPECTED", action: "dispatch", subject: "hall/mayor", at: "09:41", count: 4,
        // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
        recovery: "the watchdog suspected a loop and stopped the run; steer it elsewhere" },
    ],
  },
];

// The corner three deep, as a page that lost its link meets them. The
// fourth refusal goes to the drawer rather than into a taller stack.
export const TOASTS: readonly NoticeLine[] = [
  // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
  { code: "E_WIRE_MISMATCH", action: "connect", subject: "sprawling 0.0.6", at: "03:35", count: 1,
    // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
    recovery: "the client and the city disagree on the wire; reload after the city updates" },
  // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
  { code: "E_TIMEOUT", action: "ask", subject: "hall/mayor", at: "03:31", count: 2,
    // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
    recovery: "the city did not answer in time; ask the link to try again" },
  // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
  { code: "E_PROVIDER", action: "dispatch", subject: "api.zenmux.ai", at: "03:29", count: 1,
    // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
    recovery: "the provider failed to answer; the city kept the words for another try" },
];

// One refusal with a home: it sits under the field it was refused at,
// and leaves the moment that field is edited.
export const REFUSED_FIELD: NoticeLine = {
  code: "E_ENDPOINT_DIALECT_UNSUPPORTED",
  // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
  action: "attach",
  // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
  subject: "api_gateway.internal",
  at: "03:35",
  count: 1,
  // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
  recovery: "this endpoint speaks another dialect; pick the shape it answers in",
};

// One round of a room: the task the person sent and the answer that
// came back. `EARLIER_SEGMENT` is the round the session divider folds
// away - the same shape, one session ago.
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
  { speaker: "resident", text: "Four holdings across three shelves, and one name the room asks for that no shelf has." },
  { speaker: "person", text: "Pin the engineering ones for the next run." },
];

// The fork picker's two columns: the turns of the mother run, and what
// one of those turns did. The ids key the picker's rows and nothing
// else reads them.
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
  // As one line of the picker's right column: the call and its subject.
  readonly what: string;
  readonly at: string;
}

export const CALLS: readonly CallLine[] = [
  { id: "call-1", what: "read: city/hall/Handoff.md", at: "11:20" },
  { id: "call-2", what: "exec: just check", at: "11:24" },
  { id: "call-3", what: "edit: city/hall/PLAN.md", at: "11:31" },
  { id: "call-4", what: "exec: cargo nextest", at: "11:44" },
];

// The questions this city is holding for the person, as the cards and
// the rail's dot read them. `ONE_QUESTION` is named rather than
// reached by its position in the list, because the dot and the cards
// are two readings of the same waiting question and a reordered list
// must not make them disagree about which one that is; the presences
// fixture reads it too.
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
    created: TimeMs.make(at),
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

// Three things a person can be asked, in the three shapes the cards
// take: one on its own, several identical ones answered together, and
// one raised by a run that began with somebody else's words - which is
// never grouped with anything.
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
