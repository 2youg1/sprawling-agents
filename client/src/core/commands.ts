// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Every command frame this client sends, built in one place, so what a
// button means is written once. Each takes the person's words and mints
// its own idempotency key.

import { mintIdem } from "./idem";
export type { Endpoint, Pair, Tuning } from "./commands/endpoint";
export { attachEndpoint, probeEndpoint, reattachEndpoint } from "./commands/endpoint";
import { providerName } from "./commands/endpoint";
export { providerName };
import type {
  Address,
  AdmissionRequirement,
  ApprovalId,
  Autonomy,
  Carry,
  Ceiling,
  Command,
  Mode,
  Origin,
  SpineDocument,
  Window,
  DialectKind,
  Effort,
  GitOid,
  GovernedDocument,
  HaltScope,
  IdentityCard,
  InputKinds,
  KeepWarm,
  LandingPolicy,
  ModelTag,
  PreferencePatch,
  PursuitStep,
  Restoration,
  RunId,
  RunPolicy,
  SearchConfiguration,
  Seq,
  ToolkitSlug,
  WriteLimit,
} from "../wire";
import {
  AdmissionRequirement as AdmissionSchema,
  Ceiling as CeilingSchema,
  LandingPolicy as LandingSchema,
  WriteLimit as WriteLimitSchema,
  Effort as EffortSchema,
  Mode as ModeSchema,
  Window as WindowSchema,
  TemplateName as TemplateNameSchema,
} from "../wire";

// The levels a dispatch may ask for, in the wire's own order. The
// generated schema is the one place they are written, so a level added
// to `kernel::Effort` appears in every picker without anybody editing
// a list - and a picker cannot offer one the frame would refuse.
// Saying nothing is not on this list: it is the field left out, which
// is what `Dispatch.effort === null` spells below.
export const EFFORTS: readonly Effort[] = EffortSchema.literals;

// The modes a run may work in, in the order a control offers
// them (kernel `Mode::ALL`); the first is the one a page starts with.
export const MODES: readonly Mode[] = ModeSchema.members.map((member) => member.literal);

// The other three values of a run policy, each in the wire's order; the
// first of each is the one that adds nothing (`crates/wire/Spec.lean`
// §8-57): the full write limit, the building's own checks, the ordinary
// landing. A page starts on those, which is `FIRST_POLICY`.
export const WRITE_LIMITS: readonly WriteLimit[] = WriteLimitSchema.members.map((member) => member.literal);
export const ADMISSIONS: readonly AdmissionRequirement[] = AdmissionSchema.members.map((member) => member.literal);
export const LANDINGS: readonly LandingPolicy[] = LandingSchema.members.map((member) => member.literal);
export const FIRST_POLICY: RunPolicy = {
  mode: MODES[0] ?? "chat",
  write: WRITE_LIMITS[0] ?? "full",
  admit: ADMISSIONS[0] ?? "standing",
  landing: LANDINGS[0] ?? "ordinary",
};

// A dispatch names a room: `addr` is the room itself (`hall/mayor`),
// and the city opens no second room inside it. A room a person names on
// the building page is an address like any other (`lab/first try`,
// `core/route.ts` `roomIn`), and the first dispatch to it opens it.
export interface Dispatch {
  readonly addr: Address;
  readonly task: string;
  // What counts as done when the mode makes this a job; `statedGoal`
  // decides whether the frame carries it.
  readonly goal: string;
  // `null` when the person has chosen no level: the frame then says
  // nothing about effort and the provider decides, which is not the
  // same request as `"none"`, an instruction not to think.
  readonly effort: Effort | null;
  // The four values the run works under, as the person chose them
  // beside the box.
  readonly policy: RunPolicy;
}

// The goal a dispatch in this mode states. A chat is the person talking,
// so it states none: the city then writes no job file and the run opens
// on the words as they were typed (`city::write_brief`). Every other
// mode is work with an end, and the goal is what says where the end is.
// A goal sent with a chat put a sentence like "hello" into JOB.md and
// handed the model the goal as the message, which it answered by
// planning.
export function statedGoal(mode: Mode, goal: string): string {
  switch (mode) {
    case "chat":
      return "";
    case "work":
      return goal;
  }
}

export function dispatch(d: Dispatch): Command {
  return {
    dispatch: {
      addr: d.addr,
      task: d.task,
      goal: statedGoal(d.policy.mode, d.goal),
      policy: d.policy,
      session: null,
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

// Put one recycle-bin row back. The row's own way back travels whole,
// because a path can be discarded twice and the row knows which time.
export function restoreDiscard(restoration: Restoration): Command {
  return { restore_discard: { restoration, idem: mintIdem() } };
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

// Takes a building out of the city; its files are kept under the
// reserved subtree and its history stays in the ledger.
export function removeBuilding(addr: Address): Command {
  return { remove_building: { addr, idem: mintIdem() } };
}

// A new session at the same address: a fresh conversation in this
// room, so the frozen model and effort go and the room may choose both
// again. `carry` says whether the previous session's handoff travels
// with it; `"nothing"` is what `/new` means and `"handoff"` is what
// `--carry` asks for.
//
// `from` is what makes it a branch: the new session's first run opens
// with the conversation another run had exchanged up to that line, so
// `/fork` is this verb with an origin rather than a second one.
export function openSession(addr: Address, carry: Carry, from: Origin | null): Command {
  return { open_session: { addr, carry, from, idem: mintIdem() } };
}

// A display name for the session of `room` that began at `began`, or the
// name taken back with an empty one (kernel D22). The address stays the
// session's identity; the name is what a page draws in its place.
export function nameSession(room: Address, began: Seq, name: string): Command {
  return { name_session: { room, began, name: name.trim(), idem: mintIdem() } };
}

// The room's run policy from here on (kernel D21): the run under way
// reads it at its next safe point, and the next dispatch starts under
// it. Model and effort are not in it - they stay frozen for the session.
export function changeRunPolicy(room: Address, policy: RunPolicy): Command {
  return { change_run_policy: { room, policy, idem: mintIdem() } };
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

// What one model row states beyond its name: two ceilings, which travel
// together because a window without an output ceiling describes no model
// that can be called, and the input it takes; each absent until stated.
export interface Stated {
  readonly contextTokens: number | null;
  readonly maxOutputTokens: number | null;
  readonly input: InputKinds | null;
}

export const UNSTATED: Stated = { contextTokens: null, maxOutputTokens: null, input: null };

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
  stated: Stated = UNSTATED,
): Command {
  return {
    select_model: {
      endpoint: providerName(endpoint),
      model,
      tag,
      context_tokens: window(stated.contextTokens),
      max_output_tokens: ceiling(stated.maxOutputTokens),
      input: stated.input,
      idem: mintIdem(),
    },
  };
}

// Connecting one outside application through the broker that holds its
// OAuth. It names the application and nothing else: who this city is to
// the broker is the city's own name, which it already knows, and an id
// a person had to invent would be a step in a path meant to have none.
export function connectToolkit(toolkit: ToolkitSlug): Command {
  return { connect_toolkit: { toolkit, idem: mintIdem() } };
}

export { configureContext, configureDesktop, configureMcp, configureSandbox } from "./commands/building";

// One of the three governed documents, whole. `base` is the text the
// box started from: the raw editor and the identity cards both write
// `MAYOR.md` and `PREFERENCES.md`, so a file that moved underneath is
// refused and the draft stays in the box.
export function putDocument(which: GovernedDocument, base: string, body: string): Command {
  return { put_document: { which, base, body, idem: mintIdem() } };
}

// One identity card. The city rewrites the card's keys in `base`'s
// identity area and keeps every other byte, so the page never writes
// TOML (`crates/wire/Spec.lean` §8-59).
export function putIdentity(card: IdentityCard, base: string): Command {
  return { put_identity: { card, base, idem: mintIdem() } };
}

// A building's `RULES.toml`, whole. The city reads `body` before it
// lands and refuses one it cannot read, and a file the Mayor changed
// since `base` was read is refused rather than written over.
export function putRules(building: Address, base: string, body: string): Command {
  return { put_rules: { building, base, body, idem: mintIdem() } };
}

// One file of the city's own tree, taken back to what a checkpoint holds,
// or away when it holds none. The city refuses while a run works there.
export function restoreFile(at: Address, point: GitOid): Command {
  return { restore_file: { at, point, idem: mintIdem() } };
}

export { putGuide } from "./commands/guide";
export { closeRemoteDoor, confirmRemoteDoor, openRemoteDoor, replaceCityKey } from "./commands/door";
export { decideProposals, putRange } from "./commands/document";

// The city's own layer: `null` leaves a key as it is.
export function configureCity(keepWarm: KeepWarm | null, effort: Effort | null): Command {
  return { configure_city: { keep_warm: keepWarm, effort, idem: mintIdem() } };
}

// The city's whole `[search]`: a supplier is never sent alone, so the
// city writes what the page drew (client D94).
export function configureSearch(search: SearchConfiguration): Command {
  return { configure_city: { search, idem: mintIdem() } };
}

// One of a building's own spine documents. `base` is the text the
// person started from: these have a second writer - a resident reaches
// `Roadmap.md` through `plan` - so a file that moved underneath is
// refused rather than overwritten, and the change is written once more
// against what is there now.
export function putSpine(
  building: Address,
  which: SpineDocument,
  base: string,
  body: string,
): Command {
  return { put_spine: { building, which, base, body, idem: mintIdem() } };
}

// One named change to what this person settled about reading their
// cities. The city keeps the record in this person's own
// `~/.sprawling/config.toml`; the browser's copy in `core/prefs.ts`
// is the cache in front of it, never the authority.
export function putPreferences(patch: PreferencePatch): Command {
  return { put_preferences: { patch, idem: mintIdem() } };
}
