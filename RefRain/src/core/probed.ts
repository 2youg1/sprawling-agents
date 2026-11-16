// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import type { AxCode } from "../wire";
import type { WireApi } from "./commands";
import type { Key } from "./lang";


export interface Staged {
  readonly state: string;
  readonly detail: string | null;
  readonly figure: number | null;
}


export interface Reached {
  readonly host: string;
  readonly named: Staged;
  readonly connected: Staged;
  readonly answered: Staged;
  readonly through: Staged;
  readonly elapsedMs: number;
}


export interface ModelFact {
  readonly id: string;
  readonly contextTokens: number | null;
  readonly maxOutputTokens: number | null;
  readonly inputModalities: readonly string[];
  readonly inputPrice: string | null;
  readonly outputPrice: string | null;
}

export interface Probed {
  readonly name: string;
  
  
  readonly baseUrl: string | null;
  readonly models: readonly string[];
  readonly facts: readonly ModelFact[];
  readonly reach: Reached | null;
  
  
  readonly failure: { readonly code: string; readonly subject: string } | null;
}

function isRecord(held: unknown): held is Record<string, unknown> {
  return typeof held === "object" && held !== null && !Array.isArray(held);
}

function str(held: unknown): string | null {
  return typeof held === "string" && held !== "" ? held : null;
}


function figure(held: unknown): number | null {
  return typeof held === "number" && Number.isInteger(held) && held > 0 ? held : null;
}

function staged(held: unknown): Staged {
  if (!isRecord(held)) return { state: "", detail: null, figure: null };
  const detail = held.detail;
  return {
    state: str(held.state) ?? "",
    detail: str(detail),
    figure: typeof detail === "number" ? detail : null,
  };
}

function reached(held: unknown): Reached | null {
  if (!isRecord(held)) return null;
  const host = str(held.host);
  if (host === null) return null;
  const elapsed = held.elapsed_ms;
  return {
    host,
    named: staged(held.named),
    connected: staged(held.connected),
    answered: staged(held.answered),
    through: staged(held.through),
    elapsedMs: typeof elapsed === "number" ? elapsed : 0,
  };
}

function fact(held: unknown): ModelFact | null {
  if (!isRecord(held)) return null;
  const id = str(held.id);
  if (id === null) return null;
  const modalities = held.input_modalities;
  return {
    id,
    contextTokens: figure(held.context_tokens),
    maxOutputTokens: figure(held.max_output_tokens),
    inputModalities: Array.isArray(modalities)
      ? modalities.filter((each): each is string => typeof each === "string")
      : [],
    inputPrice: str(held.input_price),
    outputPrice: str(held.output_price),
  };
}

function failure(held: unknown): { readonly code: string; readonly subject: string } | null {
  if (!isRecord(held)) return null;
  const code = str(held.code);
  return code === null ? null : { code, subject: str(held.subject) ?? "" };
}


export function readProbed(data: Record<string, unknown>): Probed | null {
  const name = str(data.name);
  if (name === null) return null;
  const models = data.models;
  const facts = data.facts;
  return {
    name,
    baseUrl: str(data.base_url),
    models: Array.isArray(models)
      ? models.filter((each): each is string => typeof each === "string")
      : [],
    facts: Array.isArray(facts)
      ? facts.map(fact).filter((each): each is ModelFact => each !== null)
      : [],
    reach: reached(data.reach),
    failure: failure(data.failed),
  };
}


export interface Sent {
  readonly id: string;
  readonly baseUrl: string;
  readonly wireApi: WireApi;
  readonly before: Probed | null;
}


export interface Asked {
  readonly baseUrl: string;
  readonly wireApi: WireApi;
}


export function normalisedFrom(
  probed: Probed | null,
  pending: Sent | null,
  now: Asked,
): string | null {
  if (
    probed === null ||
    pending === null ||
    probed === pending.before ||
    probed.name !== pending.id
  )
    return null;
  if (now.baseUrl !== pending.baseUrl || now.wireApi !== pending.wireApi) return null;
  const recorded = probed.baseUrl;
  return recorded === null || recorded === pending.baseUrl.trim() ? null : recorded;
}


export function stoppedAt(reach: Reached, listed: Probed["failure"]): Key {
  switch (reach.named.state) {
    case "not_found":
      return "setup_reach_unknown_host";
    case "refused":
      return "setup_reach_resolver";
    default:
      break;
  }
  switch (reach.connected.state) {
    case "refused":
      return "setup_reach_refused";
    case "silent":
      return "setup_reach_silent";
    case "failed":
      return "setup_reach_socket";
    default:
      break;
  }
  switch (reach.answered.state) {
    case "name_not_usable":
      return "setup_reach_name";
    case "handshake_failed":
      return "setup_reach_tls";
    case "unreachable":
      return "setup_reach_unreachable";
    case "status":
      return byStatus(reach.answered.figure, listed);
    default:
      return "setup_reach_unreachable";
  }
}

function byStatus(status: number | null, listed: Probed["failure"]): Key {
  if (status === null) return "setup_reach_unreachable";
  if (status === 401 || status === 403)
    return listed === null ? "setup_reach_ok" : "setup_reach_key";
  if (status === 404) return "setup_reach_path";
  if (status >= 200 && status < 400) return "setup_reach_ok";
  return "setup_reach_status";
}


const UNREACHED: readonly AxCode[] = ["E_PROVIDER", "E_TIMEOUT"];


export function refusalOf(code: AxCode): "reach" | "form" {
  return UNREACHED.includes(code) ? "reach" : "form";
}
