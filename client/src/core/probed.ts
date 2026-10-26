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

import type { AxCode } from "../wire";
import type { WireApi } from "./commands";
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
  // The base URL the city wrote down for this endpoint, which is the
  // URL it will call: the city's spelling of the text the probe was
  // sent with, under the face it was sent with.
  readonly baseUrl: string | null;
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

// A probe a form sent, as it left: the id it was filed under, the box
// text and the face it was sent with, and the answer that stood in
// belief when it left.
export interface Sent {
  readonly id: string;
  readonly baseUrl: string;
  readonly wireApi: WireApi;
  readonly before: Probed | null;
}

// What the form holds now.
export interface Asked {
  readonly baseUrl: string;
  readonly wireApi: WireApi;
}

// The URL the city calls for the text a form sent, when the city wrote
// it down another way - the line a form draws under its base_url box.
//
// **This compares; it never derives.** Normalisation is one algorithm
// and it lives in `gateway::normalise_entered`, which
// `accounting::worker::credentials::Entered::resolved` runs on every
// probe and every attach. A second implementation here would be a
// second place for the rule to be wrong, so the only questions asked
// here are whether the answer is to this probe and whether two strings
// differ.
//
// The answer counts only when it is newer than the one that stood when
// the probe left and is filed under the id the probe was sent under:
// belief holds one probe answer, and until the new one arrives the slot
// still holds the last. It counts only while the box text and the face
// are the ones the probe was sent with, because the city's spelling is
// a function of those two and nothing a person typed since. The form
// never writes the answer into the box: the attach runs the same
// normalisation on the same text and face, so the box needs no help to
// be right (client/Spec.lean D102).
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

// Which stage stopped the call, as the one sentence a person can act on.
//
// Read in the order the stages happen, so the first thing that went
// wrong is what the form says: a name that never resolved explains a
// socket that never opened, and telling somebody about the socket sends
// them to look at a firewall for a typo.
//
// The staged reading asks the base URL without the key, so a 401 or a
// 403 there says only that the host wants one; whether it takes this
// key is what the model list, asked with the key, answers. A list that
// was read means the key was taken, and the sentence says so.
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

// The codes that say the far side was not reached or did not answer in
// time. Every other refusal is the city refusing what the form asked
// for, and saying "did not answer" over it sends a person to look at a
// network that answered (client D101).
const UNREACHED: readonly AxCode[] = ["E_PROVIDER", "E_TIMEOUT"];

// Which kind of refusal a code is: one of reachability, drawn beside
// the probe's reading, or the city's refusal of the form, drawn with its
// subject under the model table whose rows it is about.
export function refusalOf(code: AxCode): "reach" | "form" {
  return UNREACHED.includes(code) ? "reach" : "form";
}
