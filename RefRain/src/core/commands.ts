// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { mintIdem } from "./idem";
export type { Admission, Endpoint, Pair, Tuning } from "./commands/endpoint";
export {
  attachEndpoint,
  probeAttached,
  probeEndpoint,
  reattachEndpoint,
  saveModels,
} from "./commands/endpoint";
import { providerName } from "./commands/endpoint";
export { providerName };
import type {
  Address,
  AdmissionRequirement,
  ApprovalId,
  Autonomy,
  B3Hash,
  Carry,
  Ceiling,
  Command,
  Mode,
  Origin,
  SpineDocument,
  Window,
  DialectKind,
  Effort,
  ExposureTable,
  GitOid,
  GovernedDocument,
  HaltScope,
  IdentityCard,
  InputKinds,
  KeepWarm,
  LandingPolicy,
  ModelTag,
  PreferencePatch,
  PromptStep,
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

export const EFFORTS: readonly Effort[] = EffortSchema.literals;

export const MODES: readonly Mode[] = ModeSchema.members.map((member) => member.literal);

export const WRITE_LIMITS: readonly WriteLimit[] = WriteLimitSchema.members.map(
  (member) => member.literal,
);
export const ADMISSIONS: readonly AdmissionRequirement[] = AdmissionSchema.members.map(
  (member) => member.literal,
);
export const LANDINGS: readonly LandingPolicy[] = LandingSchema.members.map(
  (member) => member.literal,
);
export const FIRST_POLICY: RunPolicy = {
  mode: "work",
  write: WRITE_LIMITS[0] ?? "full",
  admit: ADMISSIONS[0] ?? "standing",
  landing: LANDINGS[0] ?? "ordinary",
};

export interface Dispatch {
  readonly addr: Address;
  readonly task: string;

  readonly goal: string;

  readonly effort: Effort | null;

  readonly policy: RunPolicy;
  readonly resume?: Seq | null;
  readonly images?: readonly string[];
}

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
      ...(d.resume == null ? {} : { resume: d.resume }),
      ...(d.images === undefined || d.images.length === 0 ? {} : { images: d.images }),
      effort: d.effort,
      idem: mintIdem(),
    },
  };
}

export function steer(run: RunId, text: string, images: readonly string[] = []): Command {
  return { steer: { run, text, idem: mintIdem(), ...(images.length === 0 ? {} : { images }) } };
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

export function reveal(at: Address): Command {
  return { reveal: { at, idem: mintIdem() } };
}

export function restoreDiscard(restoration: Restoration): Command {
  return { restore_discard: { restoration, idem: mintIdem() } };
}

export function doctorInstall(item: string): Command {
  return { doctor_install: { item, idem: mintIdem() } };
}

export function doctorRefresh(): Command {
  return { doctor_refresh: { idem: mintIdem() } };
}

export function approve(item: ApprovalId, verdict: "allow" | "deny"): Command {
  return { approve: { item, verdict, idem: mintIdem() } };
}

export function pursue(addr: Address, step: PursuitStep): Command {
  return { pursue: { addr, step, idem: mintIdem() } };
}

export const TEMPLATES = ["minimal", "confidential", "hall"] as const;
export type Template = (typeof TEMPLATES)[number];

export function createBuilding(
  addr: Address,
  template: Template,
  rules: string | null,
  source: string | null = null,
): Command {
  return {
    create_building: {
      addr,
      template: TemplateNameSchema.make(template),
      rules,
      ...(source === null ? {} : { source }),
      idem: mintIdem(),
    },
  };
}

export function removeBuilding(addr: Address): Command {
  return { remove_building: { addr, idem: mintIdem() } };
}

export function openSession(addr: Address, carry: Carry, from: Origin | null): Command {
  return { open_session: { addr, carry, from, idem: mintIdem() } };
}

export function nameSession(room: Address, began: Seq, name: string): Command {
  return { name_session: { room, began, name: name.trim(), idem: mintIdem() } };
}

export function changeRunPolicy(room: Address, policy: RunPolicy): Command {
  return { change_run_policy: { room, policy, idem: mintIdem() } };
}

export function setAutonomy(scope: HaltScope, autonomy: Autonomy): Command {
  return { set_autonomy: { scope, autonomy, idem: mintIdem() } };
}

export const WIRE_APIS = ["chat", "responses", "messages"] as const;
export type WireApi = (typeof WIRE_APIS)[number];

export function dialectOf(api: WireApi): DialectKind {
  switch (api) {
    case "chat":
      return "open_ai";
    case "messages":
      return "anthropic";

    case "responses":
      return "open_ai_responses";
  }
}

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

export interface Stated {
  readonly contextTokens: number | null;
  readonly maxOutputTokens: number | null;
  readonly input: InputKinds | null;
}

export const UNSTATED: Stated = { contextTokens: null, maxOutputTokens: null, input: null };

export function wireCeiling(stated: number | null): Ceiling | null {
  return stated !== null && Number.isInteger(stated) && stated > 0
    ? CeilingSchema.make(stated)
    : null;
}

export function wireWindow(stated: number | null): Window | null {
  return stated !== null && Number.isInteger(stated) && stated > 0
    ? WindowSchema.make(stated)
    : null;
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
      context_tokens: wireWindow(stated.contextTokens),
      max_output_tokens: wireCeiling(stated.maxOutputTokens),
      input: stated.input,
      idem: mintIdem(),
    },
  };
}

export function connectToolkit(toolkit: ToolkitSlug): Command {
  return { connect_toolkit: { toolkit, idem: mintIdem() } };
}

export {
  configureCatalog,
  configureContext,
  configureDesktop,
  configureMcp,
  configureSandbox,
} from "./commands/building";

export function putDocument(which: GovernedDocument, base: string, body: string): Command {
  return { put_document: { which, base, body, idem: mintIdem() } };
}

export function putPrompt(at: Address, base: B3Hash, step: PromptStep): Command {
  return { put_prompt: { at, base, step, idem: mintIdem() } };
}

export function putIdentity(card: IdentityCard, base: string): Command {
  return { put_identity: { card, base, idem: mintIdem() } };
}

export function putRules(building: Address, base: string, body: string): Command {
  return { put_rules: { building, base, body, idem: mintIdem() } };
}

export function setWorkspaceChoice(
  building: Address,
  base: string,
  endpoint: string,
  model: string,
  effort: Effort,
): Command {
  return {
    set_workspace_choice: { building, base, endpoint, model, effort, idem: mintIdem() },
  };
}

export function restoreFile(at: Address, point: GitOid): Command {
  return { restore_file: { at, point, idem: mintIdem() } };
}

export { putGuide } from "./commands/guide";
export {
  closeRemoteDoor,
  confirmRemoteDoor,
  openRemoteDoor,
  replaceCityKey,
} from "./commands/door";
export { decideProposals, putRange } from "./commands/document";
export { privacyOperation, type PrivacyOperation } from "./commands/privacy";

export function configureCity(keepWarm: KeepWarm | null, effort: Effort | null): Command {
  return { configure_city: { keep_warm: keepWarm, effort, idem: mintIdem() } };
}

export function configureSearch(search: SearchConfiguration): Command {
  return { configure_city: { search, idem: mintIdem() } };
}

export function configureCityCatalog(catalog: ExposureTable): Command {
  return { configure_city: { catalog, idem: mintIdem() } };
}

export function configureSkillShelves(shelves: readonly string[]): Command {
  return { configure_city: { skill_shelves: [...shelves], idem: mintIdem() } };
}

export function putSpine(
  building: Address,
  which: SpineDocument,
  base: string,
  body: string,
): Command {
  return { put_spine: { building, which, base, body, idem: mintIdem() } };
}

export function putPreferences(patch: PreferencePatch): Command {
  return { put_preferences: { patch, idem: mintIdem() } };
}
