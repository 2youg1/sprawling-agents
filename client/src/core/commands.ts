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
  Command,
  DialectKind,
  Effort,
  GovernedDocument,
  HaltScope,
  McpServer,
  ModelTag,
  ProviderName,
  PursuitStep,
  RunId,
  SessionName,
} from "../wire";
import { ModeTag, ProviderName as ProviderNameSchema, SessionName as SessionNameSchema } from "../wire";

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

export function approve(item: ApprovalId, verdict: "allow" | "deny"): Command {
  return { approve: { item, verdict, idem: mintIdem() } };
}

export function createPolicy(from: ApprovalId): Command {
  return { create_policy: { from_item: from, idem: mintIdem() } };
}

export function pursue(addr: Address, step: PursuitStep): Command {
  return { pursue: { addr, step, idem: mintIdem() } };
}

export function setAutonomy(scope: HaltScope, autonomy: Autonomy): Command {
  return { set_autonomy: { scope, autonomy, idem: mintIdem() } };
}

export interface Endpoint {
  readonly name: string;
  readonly baseUrl: string;
  readonly dialect: DialectKind;
  readonly secret: string | null;
  readonly authHeader: string | null;
}

export function providerName(name: string): ProviderName {
  return ProviderNameSchema.make(name);
}

export function probeEndpoint(e: Endpoint): Command {
  return {
    probe_endpoint: {
      name: providerName(e.name),
      base_url: e.baseUrl,
      dialect: e.dialect,
      secret: e.secret,
      auth_header: e.authHeader,
      idem: mintIdem(),
    },
  };
}

export function attachEndpoint(e: Endpoint, admit: readonly string[]): Command {
  return {
    attach_endpoint: {
      name: providerName(e.name),
      base_url: e.baseUrl,
      dialect: e.dialect,
      secret: e.secret,
      auth_header: e.authHeader,
      admit: [...admit],
      idem: mintIdem(),
    },
  };
}

// The two ceilings are zero: the city takes the catalogue's figure for
// the model, and a number typed on a form would outrank the one that
// bills.
export function selectModel(endpoint: string, model: string, tag: ModelTag): Command {
  return {
    select_model: {
      endpoint: providerName(endpoint),
      model,
      tag,
      context_tokens: 0,
      max_output_tokens: 0,
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

export function configureMcp(addr: Address, mcp: readonly McpServer[]): Command {
  return { configure_building: { addr, mcp: [...mcp], sandbox: null, idem: mintIdem() } };
}

export function putDocument(which: GovernedDocument, body: string): Command {
  return { put_document: { which, body, idem: mintIdem() } };
}
