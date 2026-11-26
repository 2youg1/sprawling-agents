// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Option, Schema } from "effect";

import { challenged, signed } from "./device";
import type { LocalDevice } from "./device";

export type DoorPhase = "pair" | "challenge" | "session";
export type DoorProblem = "timeout" | "network" | "response";
export interface Unreachable {
  readonly kind: "unreachable";
  readonly phase: DoorPhase;
  readonly problem: DoorProblem;
  readonly status?: number;
}

export type Knock<T> =
  | { readonly kind: "answered"; readonly value: T }
  | { readonly kind: "refused"; readonly phase: DoorPhase; readonly said: string }
  | { readonly kind: "absent" }
  | Unreachable;

const LABEL_CHARS = 64;
const DOOR_TIMEOUT_MS = 8_000;

const PairAnswer = Schema.Struct({ device: Schema.String });
const ChallengeAnswer = Schema.Struct({ nonce: Schema.String });
const SessionAnswer = Schema.Struct({ token: Schema.String });
const RecoveredAnswer = Schema.Struct({ device: Schema.String, token: Schema.String });

const ABSENT: readonly number[] = [404, 405, 501];

export async function pair(
  origin: string,
  open: string,
  publicKey: string,
  label: string,
): Promise<Knock<string>> {
  const body = { open, public_key: publicKey, label: label.slice(0, LABEL_CHARS) };
  const paired = await knock(origin, "pair", body);
  if (paired.kind !== "answered") return paired;
  return read(
    "pair",
    Schema.decodeUnknownOption(PairAnswer)(paired.value),
    (answer) => answer.device,
  );
}

export async function session(origin: string, device: LocalDevice): Promise<Knock<string>> {
  const opened = await sessionFor(origin, device, { device: device.id });
  if (opened.kind !== "answered") return opened;
  return read(
    "session",
    Schema.decodeUnknownOption(SessionAnswer)(opened.value),
    (answer) => answer.token,
  );
}

export async function recoverSession(
  origin: string,
  key: { readonly key: CryptoKey; readonly public: string },
): Promise<Knock<{ readonly device: string; readonly token: string }>> {
  const opened = await sessionFor(origin, key, { public_key: key.public });
  if (opened.kind !== "answered") return opened;
  return read(
    "session",
    Schema.decodeUnknownOption(RecoveredAnswer)(opened.value),
    (answer) => answer,
  );
}

async function sessionFor(
  origin: string,
  key: Pick<LocalDevice, "key">,
  browser: { readonly device: string } | { readonly public_key: string },
): Promise<Knock<unknown>> {
  const asked = await knock(origin, "challenge", null);
  if (asked.kind !== "answered") return asked;
  const challenge = Option.getOrNull(Schema.decodeUnknownOption(ChallengeAnswer)(asked.value));
  if (challenge === null) return { kind: "unreachable", phase: "challenge", problem: "response" };
  const { nonce } = challenge;
  const signature = await signed(key, challenged(origin, nonce));
  if (signature === null) return { kind: "refused", phase: "session", said: "" };
  return knock(origin, "session", { ...browser, nonce, signature });
}

export async function doorHere(origin: string): Promise<Knock<null>> {
  const asked = await knock(origin, "challenge", null);
  return asked.kind === "answered" ? { kind: "answered", value: null } : asked;
}

function read<A, T>(phase: DoorPhase, decoded: Option.Option<A>, take: (answer: A) => T): Knock<T> {
  return Option.match(decoded, {
    onNone: () => ({ kind: "unreachable", phase, problem: "response" }),
    onSome: (answer) => ({ kind: "answered", value: take(answer) }),
  });
}

async function knock(
  origin: string,
  phase: DoorPhase,
  body: object | null,
): Promise<Knock<unknown>> {
  const path = phase === "challenge" ? "session/challenge" : phase;
  const signal = AbortSignal.timeout(DOOR_TIMEOUT_MS);
  const response = await fetch(`${origin}/${path}`, {
    method: "POST",
    credentials: "same-origin",
    signal,
    headers: body === null ? {} : { "content-type": "application/json" },
    ...(body === null ? {} : { body: JSON.stringify(body) }),
  }).catch(() => null);
  if (response === null)
    return { kind: "unreachable", phase, problem: signal.aborted ? "timeout" : "network" };
  if (ABSENT.includes(response.status)) return { kind: "absent" };
  if (!response.ok) {
    if (response.status >= 500)
      return { kind: "unreachable", phase, problem: "response", status: response.status };
    const said = await response.text().catch(() => "");
    return { kind: "refused", phase, said };
  }
  const value: unknown = await response.json().catch(() => null);
  return value === null
    ? {
        kind: "unreachable",
        phase,
        problem: signal.aborted ? "timeout" : "response",
        status: response.status,
      }
    : { kind: "answered", value };
}
