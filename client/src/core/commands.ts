// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Every command frame this client sends, built in one place, so what a
// button means is written once. Each takes the person's words and mints
// its own idempotency key.

import { mintIdem } from "./idem";
export type { Endpoint, Pair, Tuning } from "./commands/endpoint";
export { attachEndpoint, probeEndpoint } from "./commands/endpoint";
import { providerName } from "./commands/endpoint";
export { providerName };
import type {
  Address,
  ApprovalId,
  Autonomy,
  Ceiling,
  Command,
  Mode,
  Window,
  DialectKind,
  Effort,
  GovernedDocument,
  HaltScope,
  McpServer,
  ModelTag,
  PreferencePatch,
  PursuitStep,
  RunId,
  Seq,
  SessionName,
  ToolkitSlug,
} from "../wire";
import {
  Ceiling as CeilingSchema,
  Effort as EffortSchema,
  Window as WindowSchema,
  SessionName as SessionNameSchema,
  TemplateName as TemplateNameSchema,
} from "../wire";

// The levels a dispatch may ask for, in the wire's own order. The
// generated schema is the one place they are written, so a level added
// to `kernel::Effort` appears in every picker without anybody editing
// a list - and a picker cannot offer one the frame would refuse.
// Saying nothing is not on this list: it is the field left out, which
// is what `Dispatch.effort === null` spells below.
export const EFFORTS: readonly Effort[] = EffortSchema.literals;

// The one mode a conversation runs in: plan first, then work. The city
// reads any tag it does not know as this one, so the spelling here is
// the explicit form of the default.
const PLAN_MODE: Mode = "plan_goal";

// A dispatch names a room: `addr` is the room itself (`hall/mayor`),
// and the city opens no second room inside it. Naming a session is the
// building form's business, which opens `<building>/<session>`.
export interface Dispatch {
  readonly addr: Address;
  readonly task: string;
  readonly goal: string;
  // `null` when the person has chosen no level: the frame then says
  // nothing about effort and the provider decides, which is not the
  // same request as `"none"`, an instruction not to think.
  readonly effort: Effort | null;
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

export function dialectOf(api: WireApi): DialectKind {
  switch (api) {
    case "chat":
      return "open_ai";
    case "messages":
      return "anthropic";
    // The Responses API is a third request shape rather than a second
    // spelling of Chat Completions, and the wire now carries it as
    // one, so a form no longer has to refuse the provider that speaks
    // it.
    case "responses":
      return "open_ai_responses";
  }
}

// Which `wire_api` an attached endpoint speaks, from the dialect the
// city answered with.
//
// The inverse of `dialectOf` over the values that have one, written
// beside it so the endpoint list does not spell the pair a second time
// - it used to read `dialect === "anthropic" ? "messages" : "chat"`,
// which is this mapping with the wrong answer built into its second
// half, and total in both directions now that the wire carries the
// Responses shape under a kind of its own.
export function wireApiOf(dialect: DialectKind): WireApi {
  switch (dialect) {
    case "open_ai":
      return "chat";
    case "open_ai_responses":
      return "responses";
    case "anthropic":
      return "messages";
  }
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

// A window reaches the wire only as a whole positive number, and zero
// is not one: a window nobody stated and a window of zero used to be
// the same byte, and the context reminder read every session as full.
function window(stated: number | null): Window | null {
  return stated !== null && Number.isInteger(stated) && stated > 0 ? WindowSchema.make(stated) : null;
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

// One named change to what this person settled about reading their
// cities. The city keeps the record in this person's own
// `~/.sprawling/config.toml`; the browser's copy in `core/prefs.ts`
// is the cache in front of it, never the authority.
export function putPreferences(patch: PreferencePatch): Command {
  return { put_preferences: { patch, idem: mintIdem() } };
}
