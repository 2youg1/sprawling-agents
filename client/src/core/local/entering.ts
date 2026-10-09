// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How a page opened on this machine's port comes in (client/Spec.lean
// §4-57b): with the open code `/web` put in the address's fragment,
// with the device key this origin kept, or by showing the pairing page.
// What it comes in with is a credential - the session token, renewed
// before every socket attempt - and the address bar never carries one.
//
// The fragment is read once, at the first line of the page's script,
// and taken off the address at once: a fragment is never sent to a
// server and never rides a Referer, and once the code is redeemed it
// opens nothing, but an address bar that still showed it would be
// copied into a bookmark or a screenshot.

import { forget, keep, kept, makeKey } from "./device";
import type { LocalDevice } from "./device";
import { doorHere, pair, session } from "./door";
import type { PairProof } from "./door";

// What the page holds to show the city who it is.
export interface Credential {
  // The session token in force, or null for a page that has none -
  // one opened from a file or through the remote door, which the city
  // judges as it did before.
  readonly current: () => string | null;
  // A fresh token before a socket attempt. `forgotten` when the city no
  // longer knows this device; the key is dropped by then, and the page
  // has to pair again.
  readonly renew: () => Promise<Renewal>;
}

export type Renewal = "renewed" | "kept" | "forgotten";

// Why the pairing page is drawn, which picks the sentence under its title.
export type PairWhy = "first" | "open_used" | "session_lost";

export type Entry =
  | { readonly kind: "enter"; readonly credential: Credential }
  | { readonly kind: "pair"; readonly why: PairWhy };

// What a pairing from the page came to.
export type Paired =
  | { readonly kind: "paired"; readonly credential: Credential }
  | { readonly kind: "refused"; readonly said: string }
  | { readonly kind: "unreachable" };

// The credential of a page that has no local door to knock on.
export const NO_CREDENTIAL: Credential = {
  current: () => null,
  renew: () => Promise.resolve("kept"),
};

const OPEN = "#open=";

// The open code an address's fragment carries, or null.
export function openCodeIn(hash: string): string | null {
  if (!hash.startsWith(OPEN)) return null;
  const code = hash.slice(OPEN.length);
  return code === "" ? null : code;
}

// The address once the fragment's open code is taken off: the same
// path and query, no fragment. `history.replaceState` takes this, so
// the code leaves no entry in the back list either.
export function withoutOpenCode(location: Pick<Location, "pathname" | "search">): string {
  return `${location.pathname}${location.search}`;
}

// Whether a page at this address may have a local door: only plain
// http, which is how the city prints its own address on this machine.
// A file has no door, and https is the remote door, which pairs and
// seals by its own protocol (`core/remote`).
export function mayHaveDoor(protocol: string): boolean {
  return protocol === "http:";
}

export async function enter(origin: string, open: string | null, label: string): Promise<Entry> {
  if (open !== null) {
    const redeemed = await pairWith(origin, { open }, label);
    if (redeemed.kind === "paired") return { kind: "enter", credential: redeemed.credential };
    if (redeemed.kind === "refused") return { kind: "pair", why: "open_used" };
  }
  const device = await kept();
  if (device === null) {
    const door = await doorHere(origin);
    return door.kind === "answered" ? { kind: "pair", why: "first" } : { kind: "enter", credential: NO_CREDENTIAL };
  }
  const credential = credentialOf(origin, device);
  const renewed = await credential.renew();
  return renewed === "forgotten" ? { kind: "pair", why: "session_lost" } : { kind: "enter", credential };
}

// One pairing with a code the person typed, or the open code. The key
// is made here and kept only once the city has taken its public half.
// A page that could not keep it (no IndexedDB) still comes in now and
// pairs again the next time it is opened.
export async function pairWith(origin: string, proof: PairProof, label: string): Promise<Paired> {
  const made = await makeKey();
  if (made === null) return { kind: "refused", said: "" };
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

// The credential a kept device gives: a token renewed by signing a
// fresh challenge. A city that cannot be reached keeps the last token,
// and the socket attempt that follows finds out.
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

// The name a page offers for itself, from what its user agent says:
// the browser, then the system. A person may type another on the
// pairing page; this is what the open code pairs under.
export function browserLabel(userAgent: string): string {
  const browser = BROWSERS.find(([pattern]) => pattern.test(userAgent))?.[1] ?? "Browser";
  const system = SYSTEMS.find(([pattern]) => pattern.test(userAgent))?.[1];
  return system === undefined ? browser : `${browser} · ${system}`;
}

// In the order a user agent must be read: Edge and Opera also say
// Chrome, and Chrome also says Safari.
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
