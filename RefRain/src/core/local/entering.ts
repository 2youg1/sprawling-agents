// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import { forget, keep, kept, makeKey } from "./device";
import type { LocalDevice } from "./device";
import { doorHere, pair, session } from "./door";
import type { PairProof } from "./door";


export interface Credential {
  
  
  readonly current: () => string | null;
  
  
  readonly renew: () => Promise<Renewal>;
}

export type Renewal = "renewed" | "kept" | "forgotten";


export type PairWhy = "first" | "open_used" | "session_lost" | "keyless";

export type Entry =
  | { readonly kind: "enter"; readonly credential: Credential }
  | { readonly kind: "pair"; readonly why: PairWhy };


export type Paired =
  | { readonly kind: "paired"; readonly credential: Credential }
  | { readonly kind: "refused"; readonly said: string }
  | { readonly kind: "keyless" }
  | { readonly kind: "unreachable" };


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
    const redeemed = await pairWith(origin, { open }, label);
    if (redeemed.kind === "paired") return { kind: "enter", credential: redeemed.credential };
    if (redeemed.kind === "refused") return { kind: "pair", why: "open_used" };
    if (redeemed.kind === "keyless") return { kind: "pair", why: "keyless" };
  }
  const device = await kept();
  if (device === null) {
    const door = await doorHere(origin);
    if (door.kind !== "answered") return { kind: "enter", credential: NO_CREDENTIAL };
    return { kind: "pair", why: (await makeKey()) === null ? "keyless" : "first" };
  }
  const credential = credentialOf(origin, device);
  const renewed = await credential.renew();
  return renewed === "forgotten"
    ? { kind: "pair", why: "session_lost" }
    : { kind: "enter", credential };
}


export async function pairWith(origin: string, proof: PairProof, label: string): Promise<Paired> {
  const made = await makeKey();
  if (made === null) return { kind: "keyless" };
  const answer = await pair(origin, proof, made.public, label);
  switch (answer.kind) {
    case "answered": {
      const device: LocalDevice = { id: answer.value, key: made.key };
      await keep(device);
      const credential = credentialOf(origin, device);
      await credential.renew();
      return { kind: "paired", credential };
    }
    case "refused":
      return { kind: "refused", said: answer.said };
    case "absent":
    case "unreachable":
      return { kind: "unreachable" };
  }
}


export function credentialOf(origin: string, device: LocalDevice): Credential {
  let token: string | null = null;
  return {
    current: () => token,
    renew: async () => {
      const answer = await session(origin, device);
      switch (answer.kind) {
        case "answered":
          token = answer.value;
          return "renewed";
        case "refused":
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
