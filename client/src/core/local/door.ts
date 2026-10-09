// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The three routes of the city's local door, as this page speaks them
// (`crates/wire/spec/Server.lean` §8-93): `/pair` trades an open code
// or a pairing code and a public key for a device id, and
// `/session/challenge` then `/session` trade a signature for the
// session token the page holds in memory and shows on every later
// request. Each call answers one of four readings and never throws.

import { Option, Schema } from "effect";

import { challenged, signed } from "./device";
import type { LocalDevice } from "./device";

// What one call to the door came to.
export type Knock<T> =
  | { readonly kind: "answered"; readonly value: T }
  // The door read the request and said no; `said` is its refusal text.
  | { readonly kind: "refused"; readonly said: string }
  // This listener has no local door: a page opened from a file, from
  // the remote door, or from a city built before the door existed.
  // The page speaks as it did before, and the city judges what it gets.
  | { readonly kind: "absent" }
  // Nothing answered: the city is not running, or the network broke.
  | { readonly kind: "unreachable" };

// The proof a pairing offers: exactly one of the two codes.
export type PairProof = { readonly open: string } | { readonly code: string };

// The longest name a page offers for itself; the city's list of paired
// browsers draws one line per device.
const LABEL_CHARS = 64;

const PairAnswer = Schema.Struct({ device: Schema.String });
const ChallengeAnswer = Schema.Struct({ nonce: Schema.String });
const SessionAnswer = Schema.Struct({ token: Schema.String });

// The statuses that mean the route is not there to answer: not found,
// not this method, or not built.
const ABSENT: readonly number[] = [404, 405, 501];

export async function pair(origin: string, proof: PairProof, publicKey: string, label: string): Promise<Knock<string>> {
  const body = { ...proof, public_key: publicKey, label: label.slice(0, LABEL_CHARS) };
  const paired = await knock(`${origin}/pair`, body);
  if (paired.kind !== "answered") return paired;
  return read(Schema.decodeUnknownOption(PairAnswer)(paired.value), (answer) => answer.device);
}

// A session token for `device`, or why there is none.
export async function session(origin: string, device: LocalDevice): Promise<Knock<string>> {
  const asked = await knock(`${origin}/session/challenge`, null);
  if (asked.kind !== "answered") return asked;
  const challenge = Option.getOrNull(Schema.decodeUnknownOption(ChallengeAnswer)(asked.value));
  if (challenge === null) return { kind: "refused", said: "" };
  const { nonce } = challenge;
  const signature = await signed(device, challenged(origin, nonce));
  // A key this browser can no longer sign with is a device the city
  // cannot know either; the page pairs again.
  if (signature === null) return { kind: "refused", said: "" };
  const opened = await knock(`${origin}/session`, { device: device.id, nonce, signature });
  if (opened.kind !== "answered") return opened;
  return read(Schema.decodeUnknownOption(SessionAnswer)(opened.value), (answer) => answer.token);
}

// Whether this listener has a local door at all, asked without a
// device: the challenge is the one route that needs nothing.
export async function doorHere(origin: string): Promise<Knock<null>> {
  const asked = await knock(`${origin}/session/challenge`, null);
  return asked.kind === "answered" ? { kind: "answered", value: null } : asked;
}

// An answer the door gave in a shape this page does not read is a
// refusal: nothing in it can be acted on.
function read<A>(decoded: Option.Option<A>, take: (answer: A) => string): Knock<string> {
  return Option.match(decoded, {
    onNone: () => ({ kind: "refused", said: "" }),
    onSome: (answer) => ({ kind: "answered", value: take(answer) }),
  });
}

async function knock(url: string, body: object | null): Promise<Knock<unknown>> {
  const response = await fetch(url, {
    method: "POST",
    // `same-origin` keeps any cookie a browser holds for this host out of
    // other origins' requests; the door reads only what the body says.
    credentials: "same-origin",
    headers: body === null ? {} : { "content-type": "application/json" },
    ...(body === null ? {} : { body: JSON.stringify(body) }),
  }).catch(() => null);
  if (response === null) return { kind: "unreachable" };
  if (ABSENT.includes(response.status)) return { kind: "absent" };
  if (!response.ok) {
    const said = await response.text().catch(() => "");
    return { kind: "refused", said };
  }
  const value: unknown = await response.json().catch(() => null);
  return { kind: "answered", value };
}
