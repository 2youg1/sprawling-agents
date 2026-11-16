// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { DeviceKey } from "./keys";

export interface Device {
  readonly city: Uint8Array<ArrayBuffer>;

  readonly fingerprint: Uint8Array<ArrayBuffer>;
  readonly id: Uint8Array<ArrayBuffer>;
  readonly key: DeviceKey;

  readonly at: number;
}

const BASE = "sprawling-remote";
const STORE = "device";
const ONLY = "this";

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

export async function kept(): Promise<Device | null> {
  const base = await opened();
  if (base === null) return null;
  const found = await done<unknown>(base.transaction(STORE).objectStore(STORE).get(ONLY));
  base.close();
  return isDevice(found) ? found : null;
}

export async function keep(device: Device): Promise<boolean> {
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

function isDevice(value: unknown): value is Device {
  return (
    typeof value === "object" &&
    value !== null &&
    "city" in value &&
    value.city instanceof Uint8Array &&
    "fingerprint" in value &&
    value.fingerprint instanceof Uint8Array &&
    "id" in value &&
    value.id instanceof Uint8Array &&
    "at" in value &&
    typeof value.at === "number" &&
    "key" in value &&
    isKey(value.key)
  );
}

function isKey(value: unknown): boolean {
  return (
    typeof value === "object" &&
    value !== null &&
    "ed25519" in value &&
    value.ed25519 instanceof CryptoKey &&
    "public" in value &&
    value.public instanceof Uint8Array &&
    "mlDsa" in value &&
    value.mlDsa instanceof Uint8Array
  );
}
