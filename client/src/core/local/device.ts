// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// This browser's device key for the city on this machine, in this
// origin's IndexedDB: the id the city gave it at pairing and an Ed25519
// key WebCrypto made with `extractable: false`, so no script on this
// page, this one included, can read the private half (client/Spec.lean
// §4-57b). IndexedDB rather than the preferences' rows because it keeps
// a `CryptoKey` as it is.
//
// A base of its own, apart from the remote door's `sprawling-remote`:
// the two keys answer two doors with two protocols, and forgetting one
// must not forget the other. One record per origin, because the city
// prints one address (`http://127.0.0.1:<port>`) and a browser already
// keeps origins apart; an address spelled another way is another
// origin and pairs again.

export interface LocalDevice {
  // The id the city answered `/pair` with.
  readonly id: string;
  // The private half, for signing a session challenge.
  readonly key: CryptoKey;
}

// A key made for pairing: the private half to keep, and the public half
// as `/pair` sends it.
export interface MadeKey {
  readonly key: CryptoKey;
  readonly public: string;
}

const BASE = "sprawling-local";
const STORE = "device";
const ONLY = "this";

// A fresh device key, or null where this browser's WebCrypto has no
// Ed25519, which leaves it unable to pair at all.
export async function makeKey(): Promise<MadeKey | null> {
  const pair = await crypto.subtle.generateKey({ name: "Ed25519" }, false, ["sign"]).catch(() => null);
  if (pair === null || !("privateKey" in pair)) return null;
  const raw = await crypto.subtle.exportKey("raw", pair.publicKey).catch(() => null);
  return raw === null ? null : { key: pair.privateKey, public: base64url(new Uint8Array(raw)) };
}

// The signature `/session` asks for, over the message `challenged` spells.
export async function signed(device: LocalDevice, message: Uint8Array<ArrayBuffer>): Promise<string | null> {
  const signature = await crypto.subtle.sign({ name: "Ed25519" }, device.key, message).catch(() => null);
  return signature === null ? null : base64url(new Uint8Array(signature));
}

// The bytes a device signs to open a session: a label that names this
// protocol, the page's origin, and the nonce the city just handed out,
// one per line. The origin binds the signature to the listener that
// asked, so a page on another port cannot replay it.
export function challenged(origin: string, nonce: string): Uint8Array<ArrayBuffer> {
  return new TextEncoder().encode(`sprawling local session v1\n${origin}\n${nonce}`);
}

// Unpadded base64url, the form a JWK writes a key in.
export function base64url(bytes: Uint8Array): string {
  return btoa(String.fromCharCode(...bytes)).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function opened(): Promise<IDBDatabase | null> {
  if (typeof indexedDB === "undefined") return Promise.resolve(null);
  return new Promise((settle) => {
    const asked = indexedDB.open(BASE, 1);
    asked.onupgradeneeded = () => {
      asked.result.createObjectStore(STORE);
    };
    asked.onsuccess = () => {
      settle(asked.result);
    };
    asked.onerror = () => {
      settle(null);
    };
    asked.onblocked = () => {
      settle(null);
    };
  });
}

function done<T>(request: IDBRequest<T>): Promise<T | null> {
  return new Promise((settle) => {
    request.onsuccess = () => {
      settle(request.result);
    };
    request.onerror = () => {
      settle(null);
    };
  });
}

// The device this origin paired, or null for none (or no IndexedDB).
export async function kept(): Promise<LocalDevice | null> {
  const base = await opened();
  if (base === null) return null;
  const found = await done<unknown>(base.transaction(STORE).objectStore(STORE).get(ONLY));
  base.close();
  return isDevice(found) ? found : null;
}

// Whether the record was written. A browser that keeps nothing (a
// private window with storage refused) still pairs; it pairs again the
// next time it opens the page.
export async function keep(device: LocalDevice): Promise<boolean> {
  const base = await opened();
  if (base === null) return false;
  const written = await done(base.transaction(STORE, "readwrite").objectStore(STORE).put(device, ONLY));
  base.close();
  return written !== null;
}

// Whether the record is gone. Called when the city no longer knows the
// device, so the next opening shows the pairing page instead of failing
// the same signature again.
export async function forget(): Promise<boolean> {
  const base = await opened();
  if (base === null) return false;
  const removed = await done(base.transaction(STORE, "readwrite").objectStore(STORE).delete(ONLY));
  base.close();
  return removed !== null;
}

// A record read back is whatever the store held; it is a device only if
// every field has the shape this module wrote.
function isDevice(value: unknown): value is LocalDevice {
  return (
    typeof value === "object" &&
    value !== null &&
    "id" in value &&
    typeof value.id === "string" &&
    "key" in value &&
    value.key instanceof CryptoKey
  );
}
