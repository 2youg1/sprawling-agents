// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { forget, keep, keepPairing, kept, makeKey, pendingPairing } from "./device";
import type { LocalDevice, MadeKey } from "./device";
import { doorHere, pair, recoverSession, session } from "./door";
import type { DoorPhase, DoorProblem, Knock, Unreachable } from "./door";

export interface Credential {
  readonly current: () => string | null;

  readonly renew: () => Promise<Renewal>;
}

export type Renewal = "renewed" | "kept" | "forgotten";

export type PairWhy =
  | "first"
  | "open_used"
  | "session_lost"
  | "keyless"
  | "pair_unknown"
  | "pair_unconfirmed"
  | "session_pending"
  | "unreachable";

export interface LocalDoorIssue {
  readonly phase: DoorPhase;
  readonly problem: DoorProblem | "refused" | "unconfirmed";
  readonly status?: number;
}

interface Recovery {
  readonly issue?: LocalDoorIssue | undefined;
  readonly check?: (() => Promise<Paired>) | undefined;
}

export type Entry =
  | { readonly kind: "enter"; readonly credential: Credential }
  | ({ readonly kind: "pair"; readonly why: PairWhy } & Recovery);

export type Paired =
  | { readonly kind: "paired"; readonly credential: Credential }
  | { readonly kind: "refused"; readonly said: string }
  | { readonly kind: "keyless" }
  | { readonly kind: "session_lost" }
  | ({
      readonly kind: "pair_unknown" | "pair_unconfirmed" | "session_pending" | "unreachable";
    } & Recovery);

export const NO_CREDENTIAL: Credential = {
  current: () => null,
  renew: () => Promise.resolve("kept"),
};

const OPEN = "#open=";

export function openCodeIn(hash: string): string | null {
  if (!hash.startsWith(OPEN)) return null;
  const code = hash.slice(OPEN.length);
  return code === "" ? null : code;
}

export function withoutOpenCode(location: Pick<Location, "pathname" | "search">): string {
  return `${location.pathname}${location.search}`;
}

export function mayHaveDoor(protocol: string): boolean {
  return protocol === "http:";
}

export async function enter(origin: string, open: string | null, label: string): Promise<Entry> {
  if (open !== null) {
    return arrival(await pairWith(origin, open, label));
  }
  const device = await kept();
  if (device !== null) return arrival(await resume(origin, device));
  const pending = await pendingPairing();
  if (pending !== null) return arrival(await confirmPairing(origin, pending));
  const door = await doorHere(origin);
  if (door.kind === "absent") return { kind: "enter", credential: NO_CREDENTIAL };
  if (door.kind !== "answered")
    return {
      kind: "pair",
      why: "unreachable",
      issue: issueOf("challenge", door),
    };
  return { kind: "pair", why: (await makeKey()) === null ? "keyless" : "first" };
}

export async function pairWith(origin: string, open: string, label: string): Promise<Paired> {
  const made = await makeKey();
  if (made === null) return { kind: "keyless" };
  await keepPairing(made);
  const answer = await pair(origin, open, made.public, label);
  switch (answer.kind) {
    case "answered": {
      const device: LocalDevice = { id: answer.value, key: made.key };
      if (await keep(device)) await keepPairing(null);
      return resume(origin, device);
    }
    case "refused": {
      await keepPairing(null);
      return { kind: "refused", said: answer.said };
    }
    case "absent":
      return { kind: "unreachable", issue: { phase: "pair", problem: "response" } };
    case "unreachable":
      return confirmPairing(origin, made, answer);
  }
}

function arrival(result: Paired): Entry {
  switch (result.kind) {
    case "paired":
      return { kind: "enter", credential: result.credential };
    case "refused":
      return { kind: "pair", why: "open_used" };
    case "keyless":
    case "session_lost":
      return { kind: "pair", why: result.kind };
    case "pair_unknown":
    case "pair_unconfirmed":
    case "session_pending":
    case "unreachable":
      return { kind: "pair", why: result.kind, issue: result.issue, check: result.check };
  }
}

async function confirmPairing(origin: string, made: MadeKey, lost?: Unreachable): Promise<Paired> {
  const found = await recoverSession(origin, made);
  if (found.kind === "answered") {
    const device = { id: found.value.device, key: made.key };
    if (await keep(device)) await keepPairing(null);
    return { kind: "paired", credential: credentialOf(origin, device, found.value.token) };
  }
  const check = () => confirmPairing(origin, made);
  if (missing(found))
    return {
      kind: "pair_unconfirmed",
      issue: { phase: "pair", problem: "unconfirmed" },
      check,
    };
  return { kind: "pair_unknown", issue: lost ?? issueOf("session", found), check };
}

async function resume(origin: string, device: LocalDevice): Promise<Paired> {
  const answer = await session(origin, device);
  if (answer.kind === "answered")
    return { kind: "paired", credential: credentialOf(origin, device, answer.value) };
  if (missing(answer)) {
    await forget();
    return { kind: "session_lost" };
  }
  return {
    kind: "session_pending",
    issue: issueOf("session", answer),
    check: () => resume(origin, device),
  };
}

function issueOf(
  phase: DoorPhase,
  failed: Exclude<Knock<unknown>, { readonly kind: "answered" }>,
): LocalDoorIssue {
  if (failed.kind === "unreachable") return failed;
  if (failed.kind === "refused") return { phase: failed.phase, problem: "refused" };
  return { phase, problem: "response" };
}

function missing(answer: Knock<unknown>): boolean {
  return (
    answer.kind === "refused" &&
    answer.phase === "session" &&
    (answer.said.includes("the city does not know it") ||
      answer.said.includes("its signature does not verify"))
  );
}

export function credentialOf(origin: string, device: LocalDevice, first?: string): Credential {
  let token = first ?? null;
  return {
    current: () => token,
    renew: async () => {
      const answer = await session(origin, device);
      switch (answer.kind) {
        case "answered":
          token = answer.value;
          return "renewed";
        case "refused":
          if (!missing(answer)) return "kept";
          token = null;
          await forget();
          return "forgotten";
        case "absent":
        case "unreachable":
          return "kept";
      }
    },
  };
}

export function browserLabel(userAgent: string): string {
  const browser = BROWSERS.find(([pattern]) => pattern.test(userAgent))?.[1] ?? "Browser";
  const system = SYSTEMS.find(([pattern]) => pattern.test(userAgent))?.[1];
  return system === undefined ? browser : `${browser} · ${system}`;
}

const BROWSERS: readonly (readonly [RegExp, string])[] = [
  [/Edg\//, "Edge"],
  [/OPR\//, "Opera"],
  [/Firefox\//, "Firefox"],
  [/Chrome\//, "Chrome"],
  [/Safari\//, "Safari"],
];

const SYSTEMS: readonly (readonly [RegExp, string])[] = [
  [/Windows/, "Windows"],
  [/iPhone|iPad/, "iOS"],
  [/Mac OS X|Macintosh/, "macOS"],
  [/Android/, "Android"],
  [/Linux/, "Linux"],
];
