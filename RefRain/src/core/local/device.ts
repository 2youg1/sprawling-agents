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
const PENDING = "pairing";
const STORAGE_TIMEOUT_MS = 4_000;

export async function makeKey(): Promise<MadeKey | null> {
  const subtle = globalThis.crypto?.subtle;
  if (subtle === undefined) return null;
  const pair = await subtle.generateKey({ name: "Ed25519" }, false, ["sign"]).catch(() => null);
  if (pair === null || !("privateKey" in pair)) return null;
  const raw = await subtle.exportKey("raw", pair.publicKey).catch(() => null);
  return raw === null ? null : { key: pair.privateKey, public: hex(new Uint8Array(raw)) };
}

export async function signed(
  device: Pick<LocalDevice, "key">,
  message: Uint8Array<ArrayBuffer>,
): Promise<string | null> {
  const subtle = globalThis.crypto?.subtle;
  if (subtle === undefined) return null;
  const signature = await subtle.sign({ name: "Ed25519" }, device.key, message).catch(() => null);
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
    let active = true;
    const deadline = setTimeout(() => finish(null), STORAGE_TIMEOUT_MS);
    function finish(base: IDBDatabase | null): void {
      if (!active) {
        base?.close();
        return;
      }
      active = false;
      clearTimeout(deadline);
      settle(base);
    }
    asked.onupgradeneeded = () => {
      asked.result.createObjectStore(STORE);
    };
    asked.onsuccess = () => {
      finish(asked.result);
    };
    asked.onerror = () => {
      finish(null);
    };
    asked.onblocked = () => {
      finish(null);
    };
  });
}

function done<T>(request: IDBRequest<T>): Promise<T | null> {
  return new Promise((settle) => {
    const deadline = setTimeout(() => settle(null), STORAGE_TIMEOUT_MS);
    request.onsuccess = () => {
      clearTimeout(deadline);
      settle(request.result);
    };
    request.onerror = () => {
      clearTimeout(deadline);
      settle(null);
    };
  });
}

async function heldAt(slot: string): Promise<unknown> {
  const base = await opened();
  if (base === null) return null;
  const found = await done<unknown>(base.transaction(STORE).objectStore(STORE).get(slot));
  base.close();
  return found;
}

async function writeAt(slot: string, value: LocalDevice | MadeKey | null): Promise<boolean> {
  const base = await opened();
  if (base === null) return false;
  const transaction = base.transaction(STORE, "readwrite");
  const store = transaction.objectStore(STORE);
  const written = new Promise<boolean>((settle) => {
    const deadline = setTimeout(() => {
      settle(false);
      void Promise.resolve()
        .then(() => transaction.abort())
        .catch(() => undefined);
    }, STORAGE_TIMEOUT_MS);
    function finish(result: boolean): void {
      clearTimeout(deadline);
      settle(result);
    }
    transaction.oncomplete = () => finish(true);
    transaction.onabort = () => finish(false);
  });
  if (value === null) store.delete(slot);
  else store.put(value, slot);
  const kept = await written;
  base.close();
  return kept;
}

export async function kept(): Promise<LocalDevice | null> {
  const found = await heldAt(ONLY);
  return isDevice(found) ? found : null;
}

export function keep(device: LocalDevice): Promise<boolean> {
  return writeAt(ONLY, device);
}

export function forget(): Promise<boolean> {
  return writeAt(ONLY, null);
}

export async function pendingPairing(): Promise<MadeKey | null> {
  const found = await heldAt(PENDING);
  return hasKey(found) && "public" in found && typeof found.public === "string"
    ? { key: found.key, public: found.public }
    : null;
}

export function keepPairing(key: MadeKey | null): Promise<boolean> {
  return writeAt(PENDING, key);
}

function hasKey(value: unknown): value is { readonly key: CryptoKey } {
  return (
    typeof value === "object" && value !== null && "key" in value && value.key instanceof CryptoKey
  );
}

function isDevice(value: unknown): value is LocalDevice {
  return hasKey(value) && "id" in value && typeof value.id === "string";
}
