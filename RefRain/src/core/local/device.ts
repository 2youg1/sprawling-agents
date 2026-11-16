// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export interface LocalDevice {
  readonly id: string;

  readonly key: CryptoKey;
}

export interface MadeKey {
  readonly key: CryptoKey;
  readonly public: string;
}

const BASE = "sprawling-local";
const STORE = "device";
const ONLY = "this";

export async function makeKey(): Promise<MadeKey | null> {
  const pair = await crypto.subtle
    .generateKey({ name: "Ed25519" }, false, ["sign"])
    .catch(() => null);
  if (pair === null || !("privateKey" in pair)) return null;
  const raw = await crypto.subtle.exportKey("raw", pair.publicKey).catch(() => null);
  return raw === null ? null : { key: pair.privateKey, public: hex(new Uint8Array(raw)) };
}

export async function signed(
  device: LocalDevice,
  message: Uint8Array<ArrayBuffer>,
): Promise<string | null> {
  const signature = await crypto.subtle
    .sign({ name: "Ed25519" }, device.key, message)
    .catch(() => null);
  return signature === null ? null : hex(new Uint8Array(signature));
}

export function challenged(origin: string, nonce: string): Uint8Array<ArrayBuffer> {
  return new TextEncoder().encode(`sprawling local session v1\n${nonce}\n${origin}`);
}

export function hex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
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

export async function kept(): Promise<LocalDevice | null> {
  const base = await opened();
  if (base === null) return null;
  const found = await done<unknown>(base.transaction(STORE).objectStore(STORE).get(ONLY));
  base.close();
  return isDevice(found) ? found : null;
}

export async function keep(device: LocalDevice): Promise<boolean> {
  const base = await opened();
  if (base === null) return false;
  const written = await done(
    base.transaction(STORE, "readwrite").objectStore(STORE).put(device, ONLY),
  );
  base.close();
  return written !== null;
}

export async function forget(): Promise<boolean> {
  const base = await opened();
  if (base === null) return false;
  const removed = await done(base.transaction(STORE, "readwrite").objectStore(STORE).delete(ONLY));
  base.close();
  return removed !== null;
}

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
