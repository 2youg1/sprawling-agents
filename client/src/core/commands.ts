// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Every command frame this client sends, built in one place, so what a
// button means is written once. Each takes the person's words and mints
// its own idempotency key.

import { mintIdem } from "./idem";
import type {
  Address,
  ApprovalId,
  Autonomy,
  Ceiling,
  Command,
  DialectKind,
  Effort,
  EndpointTuning,
  Proxying,
  GovernedDocument,
  HaltScope,
  McpServer,
  ModelTag,
  ProviderName,
  PursuitStep,
  RunId,
  Seq,
  SessionName,
  ToolkitSlug,
} from "../wire";
import {
  Ceiling as CeilingSchema,
  ModeTag,
  ProviderName as ProviderNameSchema,
  SessionName as SessionNameSchema,
  TemplateName as TemplateNameSchema,
} from "../wire";

// The one mode a conversation runs in: plan first, then work. The city
// reads any tag it does not know as this one, so the spelling here is
// the explicit form of the default.
const PLAN_MODE: ModeTag = ModeTag.make("plan_goal");

// A dispatch names a room: `addr` is the room itself (`hall/mayor`),
// and the city opens no second room inside it. Naming a session is the
// building form's business, which opens `<building>/<session>`.
export interface Dispatch {
  readonly addr: Address;
  readonly task: string;
  readonly goal: string;
  readonly effort: Effort;
}

export function dispatch(d: Dispatch): Command {
  return {
    dispatch: {
      addr: d.addr,
      task: d.task,
      goal: d.goal,
      mode: PLAN_MODE,
      session: null,
      effort: d.effort,
      idem: mintIdem(),
    },
  };
}

// A new session in a building: the city opens `<building>/<session>`.
export function open(building: Address, session: string, d: Omit<Dispatch, "addr">): Command {
  const named: SessionName = SessionNameSchema.make(session);
  return {
    dispatch: {
      addr: building,
      task: d.task,
      goal: d.goal,
      mode: PLAN_MODE,
      session: named,
      effort: d.effort,
      idem: mintIdem(),
    },
  };
}

export function steer(run: RunId, text: string): Command {
  return { steer: { run, text, idem: mintIdem() } };
}

export function cancel(run: RunId): Command {
  return { cancel: { run, idem: mintIdem() } };
}

export function halt(scope: HaltScope): Command {
  return { halt: { scope, idem: mintIdem() } };
}

export function release(scope: HaltScope): Command {
  return { release: { scope, idem: mintIdem() } };
}

// Show a path where the person keeps their files. The address grammar is
// the guard on the other side: nothing outside the city can be spelled.
export function reveal(at: Address): Command {
  return { reveal: { at, idem: mintIdem() } };
}

// Install one thing this machine lacks, by the name the city answered
// with. Only a recipe the city may run is run; the other two come back
// as a refusal saying what the person does instead.
export function doctorInstall(item: string): Command {
  return { doctor_install: { item, idem: mintIdem() } };
}

// Look at this machine again. `Query::Doctor` answers the snapshot the
// city took when it started, which is the wrong answer to give somebody
// who has just installed something.
export function doctorRefresh(): Command {
  return { doctor_refresh: { idem: mintIdem() } };
}

export function approve(item: ApprovalId, verdict: "allow" | "deny"): Command {
  return { approve: { item, verdict, idem: mintIdem() } };
}

export function pursue(addr: Address, step: PursuitStep): Command {
  return { pursue: { addr, step, idem: mintIdem() } };
}

// The three layouts a new building can start with; the city refuses any
// other name and says these three back, so the list is here only to keep
// a typo out of a round trip.
export const TEMPLATES = ["minimal", "confidential", "hall"] as const;
export type Template = (typeof TEMPLATES)[number];

export function createBuilding(addr: Address, template: Template): Command {
  return {
    create_building: {
      addr,
      template: TemplateNameSchema.make(template),
      idem: mintIdem(),
    },
  };
}

// A second line from a point on an existing one. `addr` is null for a
// fork that stays in the room it came from, which is the only form the
// palette offers: forking somewhere else is a move, and a move is the
// dispatch form's business.
export function fork(run: RunId, atSeq: Seq, addr: Address | null): Command {
  return { fork: { run, at_seq: atSeq, addr, idem: mintIdem() } };
}

export function setAutonomy(scope: HaltScope, autonomy: Autonomy): Command {
  return { set_autonomy: { scope, autonomy, idem: mintIdem() } };
}

// The three wire APIs a provider states in Codex's `config.toml`, in
// that file's own spelling. `DialectKind` stays the internal word: two
// of the three translate to one, and the third has no translation yet,
// so the mapping is a total function that answers with absence rather
// than with a guess.
export const WIRE_APIS = ["chat", "responses", "messages"] as const;
export type WireApi = (typeof WIRE_APIS)[number];

export function dialectOf(api: WireApi): DialectKind | null {
  switch (api) {
    case "chat":
      return "open_ai";
    case "messages":
      return "anthropic";
    // The Responses API is a third request shape rather than a second
    // spelling of Chat Completions; this city's wire cannot carry it,
    // and a form that sent `open_ai` for it would send the wrong body.
    case "responses":
      return null;
  }
}

export interface Endpoint {
  // `[model_providers.<id>]` in Codex's config.toml: the key the
  // credential reference is derived from, and the name the city files
  // the endpoint under.
  readonly id: string;
  readonly baseUrl: string;
  readonly dialect: DialectKind;
  readonly secret: string | null;
  readonly authHeader: string | null;
  readonly tuning: Tuning;
}

// One row of either key-value table the form draws: a header every
// request carries, or a JSON pointer into the body it sends.
export interface Pair {
  readonly name: string;
  readonly value: string;
}

// What a person settled about one endpoint besides its address, in the
// spelling Codex's `[model_providers.<id>]` uses. Every figure is
// absent until somebody states one, and the city reads an absent figure
// as its own default rather than as zero.
export interface Tuning {
  readonly label: string | null;
  readonly timeoutMs: number | null;
  readonly requestMaxRetries: number | null;
  readonly streamIdleTimeoutMs: number | null;
  readonly headers: readonly Pair[];
  readonly overrides: readonly Pair[];
  // Which of this endpoint's calls go through the machine's proxy. The
  // city's own rule keeps a call to an address on this machine off it,
  // and that is what `null` asks for.
  readonly proxying: Proxying | null;
}

// A figure reaches the wire only as a whole number that is not
// negative. A retry count of zero is a real answer - "once, then report"
// - so it travels, while every deadline is refused at zero: no request
// completes in no time, and a cleared box means "the city's own".
function count(stated: number | null): number | null {
  return stated !== null && Number.isInteger(stated) && stated >= 0 ? stated : null;
}

function span(stated: number | null): number | null {
  const whole = count(stated);
  return whole !== null && whole > 0 ? whole : null;
}

// The tuning as the frame carries it. A row whose name is blank is left
// out: the form keeps an empty row open while somebody types into it,
// and a half-written header must not reach a provider.
function tuningFrame(tuning: Tuning): EndpointTuning {
  const named = (rows: readonly Pair[]) => rows.filter((row) => row.name.trim() !== "");
  return {
    label: tuning.label === null || tuning.label.trim() === "" ? null : tuning.label.trim(),
    timeout_ms: span(tuning.timeoutMs),
    request_max_retries: count(tuning.requestMaxRetries),
    stream_idle_timeout_ms: span(tuning.streamIdleTimeoutMs),
    headers: named(tuning.headers).map((row) => ({ name: row.name.trim(), value: row.value })),
    overrides: named(tuning.overrides).map((row) => ({ pointer: row.name.trim(), value: row.value })),
    proxying: tuning.proxying,
  };
}

export function providerName(name: string): ProviderName {
  return ProviderNameSchema.make(name);
}

export function probeEndpoint(e: Endpoint): Command {
  return {
    probe_endpoint: {
      name: providerName(e.id),
      base_url: e.baseUrl,
      dialect: e.dialect,
      secret: e.secret,
      auth_header: e.authHeader,
      tuning: tuningFrame(e.tuning),
      idem: mintIdem(),
    },
  };
}

export function attachEndpoint(e: Endpoint, admit: readonly string[]): Command {
  return {
    attach_endpoint: {
      name: providerName(e.id),
      base_url: e.baseUrl,
      dialect: e.dialect,
      secret: e.secret,
      auth_header: e.authHeader,
      admit: [...admit],
      tuning: tuningFrame(e.tuning),
      idem: mintIdem(),
    },
  };
}

// The two ceilings one model row states. They travel together because a
// context window without an output ceiling describes no model that can
// be called, and both are absent until somebody states them: the
// catalogue holds two rows, so a provider outside it is only as good as
// the figures a person read off its own model list.
export interface Ceilings {
  readonly contextTokens: number | null;
  readonly maxOutputTokens: number | null;
}

export const UNSTATED: Ceilings = { contextTokens: null, maxOutputTokens: null };

// A ceiling reaches the wire only as a whole positive number. Anything
// else is nobody's figure, and absence is what the city reads as "take
// the catalogue's": a zero would reach the provider as `max_tokens: 0`,
// which answers with nothing.
function ceiling(stated: number | null): Ceiling | null {
  return stated !== null && Number.isInteger(stated) && stated > 0 ? CeilingSchema.make(stated) : null;
}

function window(stated: number | null): number {
  return stated !== null && Number.isInteger(stated) && stated > 0 ? stated : 0;
}

export function selectModel(
  endpoint: string,
  model: string,
  tag: ModelTag,
  ceilings: Ceilings = UNSTATED,
): Command {
  return {
    select_model: {
      endpoint: providerName(endpoint),
      model,
      tag,
      context_tokens: window(ceilings.contextTokens),
      max_output_tokens: ceiling(ceilings.maxOutputTokens),
      idem: mintIdem(),
    },
  };
}

export function loginBegin(provider: string): Command {
  return { login: { provider: providerName(provider), step: "begin", idem: mintIdem() } };
}

export function loginCode(provider: string, code: string): Command {
  return {
    login: { provider: providerName(provider), step: { code: { code } }, idem: mintIdem() },
  };
}

// Connecting one outside application through the broker that holds its
// OAuth. It names the application and nothing else: who this city is to
// the broker is the city's own name, which it already knows, and an id
// a person had to invent would be a step in a path meant to have none.
export function connectToolkit(toolkit: ToolkitSlug): Command {
  return { connect_toolkit: { toolkit, idem: mintIdem() } };
}

export function configureMcp(addr: Address, mcp: readonly McpServer[]): Command {
  return {
    configure_building: {
      addr,
      mcp: [...mcp],
      sandbox: null,
      desktop: null,
      idem: mintIdem(),
    },
  };
}

// The windows on this person's own machine a building's connector may
// touch. Sent as text, because the connector that reads the file is the
// authority on its syntax and this page must not become a second one.
export function configureDesktop(addr: Address, allowlist: string): Command {
  return {
    configure_building: {
      addr,
      mcp: null,
      sandbox: null,
      desktop: allowlist,
      idem: mintIdem(),
    },
  };
}

export function putDocument(which: GovernedDocument, body: string): Command {
  return { put_document: { which, body, idem: mintIdem() } };
}
