// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What an `endpoint_probed` record says, read once.
//
// The record is a payload rather than a typed answer, so somebody has to
// narrow it. Doing that in the fold would put four readings in a
// function whose job is to move one field, and doing it in the view
// would put them in every view that shows a probe. It happens here, and
// what leaves this file is typed.
//
// **Absent is a value.** A provider's model list states a window, a
// ceiling, modalities and prices under names each vendor chose, and most
// rows state none of them. A reading that filled in a zero would put a
// number in front of a person that no provider agreed to.

import type { Key } from "./lang";

// One stage of a staged reading, as `kernel::reach` writes it: a word
// for the state, and whatever that word carries - a message for the
// stages that failed with one, a count or a status for the stages that
// answered with a number.
export interface Staged {
  readonly state: string;
  readonly detail: string | null;
  readonly figure: number | null;
}

// Where a call to one host stopped, stage by stage.
export interface Reached {
  readonly host: string;
  readonly named: Staged;
  readonly connected: Staged;
  readonly answered: Staged;
  readonly through: Staged;
  readonly elapsedMs: number;
}

// What one row of the provider's model list stated. Everything but the
// id is absent for most providers.
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
  readonly models: readonly string[];
  readonly facts: readonly ModelFact[];
  readonly reach: Reached | null;
  // Why the model list could not be read, when it could not. The staged
  // reading says where the call stopped; this says what the city called
  // that.
  readonly failure: { readonly code: string; readonly subject: string } | null;
}

function isRecord(held: unknown): held is Record<string, unknown> {
  return typeof held === "object" && held !== null && !Array.isArray(held);
}

function str(held: unknown): string | null {
  return typeof held === "string" && held !== "" ? held : null;
}

// A whole positive number, or nothing. A zero window and a zero ceiling
// both mean "nobody stated this", which is the one reading that keeps a
// request from carrying `max_tokens: 0`.
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

// One `endpoint_probed` payload, or nothing when it does not name an
// endpoint - which no record this build writes does.
export function readProbed(data: Record<string, unknown>): Probed | null {
  const name = str(data.name);
  if (name === null) return null;
  const models = data.models;
  const facts = data.facts;
  return {
    name,
    models: Array.isArray(models) ? models.filter((each): each is string => typeof each === "string") : [],
    facts: Array.isArray(facts)
      ? facts.map(fact).filter((each): each is ModelFact => each !== null)
      : [],
    reach: reached(data.reach),
    failure: failure(data.failed),
  };
}

// Which stage stopped the call, as the one sentence a person can act on.
//
// Read in the order the stages happen, so the first thing that went
// wrong is what the form says: a name that never resolved explains a
// socket that never opened, and telling somebody about the socket sends
// them to look at a firewall for a typo.
export function stoppedAt(reach: Reached): Key {
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
      return byStatus(reach.answered.figure);
    default:
      return "setup_reach_unreachable";
  }
}

function byStatus(status: number | null): Key {
  if (status === null) return "setup_reach_unreachable";
  if (status === 401 || status === 403) return "setup_reach_key";
  if (status === 404) return "setup_reach_path";
  if (status >= 200 && status < 400) return "setup_reach_ok";
  return "setup_reach_status";
}
