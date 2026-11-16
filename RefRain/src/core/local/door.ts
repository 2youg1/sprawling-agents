// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Option, Schema } from "effect";

import { challenged, signed } from "./device";
import type { LocalDevice } from "./device";

export type Knock<T> =
  | { readonly kind: "answered"; readonly value: T }
  | { readonly kind: "refused"; readonly said: string }
  | { readonly kind: "absent" }
  | { readonly kind: "unreachable" };

export type PairProof = { readonly open: string } | { readonly code: string };

const LABEL_CHARS = 64;

const PairAnswer = Schema.Struct({ device: Schema.String });
const ChallengeAnswer = Schema.Struct({ nonce: Schema.String });
const SessionAnswer = Schema.Struct({ token: Schema.String });

const ABSENT: readonly number[] = [404, 405, 501];

export async function pair(
  origin: string,
  proof: PairProof,
  publicKey: string,
  label: string,
): Promise<Knock<string>> {
  const body = { ...proof, public_key: publicKey, label: label.slice(0, LABEL_CHARS) };
  const paired = await knock(`${origin}/pair`, body);
  if (paired.kind !== "answered") return paired;
  return read(Schema.decodeUnknownOption(PairAnswer)(paired.value), (answer) => answer.device);
}

export async function session(origin: string, device: LocalDevice): Promise<Knock<string>> {
  const asked = await knock(`${origin}/session/challenge`, null);
  if (asked.kind !== "answered") return asked;
  const challenge = Option.getOrNull(Schema.decodeUnknownOption(ChallengeAnswer)(asked.value));
  if (challenge === null) return { kind: "refused", said: "" };
  const { nonce } = challenge;
  const signature = await signed(device, challenged(origin, nonce));
  if (signature === null) return { kind: "refused", said: "" };
  const opened = await knock(`${origin}/session`, { device: device.id, nonce, signature });
  if (opened.kind !== "answered") return opened;
  return read(Schema.decodeUnknownOption(SessionAnswer)(opened.value), (answer) => answer.token);
}

export async function doorHere(origin: string): Promise<Knock<null>> {
  const asked = await knock(`${origin}/session/challenge`, null);
  return asked.kind === "answered" ? { kind: "answered", value: null } : asked;
}

function read<A>(decoded: Option.Option<A>, take: (answer: A) => string): Knock<string> {
  return Option.match(decoded, {
    onNone: () => ({ kind: "refused", said: "" }),
    onSome: (answer) => ({ kind: "answered", value: take(answer) }),
  });
}

async function knock(url: string, body: object | null): Promise<Knock<unknown>> {
  const response = await fetch(url, {
    method: "POST",
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
