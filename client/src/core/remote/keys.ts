// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export const SEED_BYTES = 32;
export interface DeviceKey {
  readonly public: Uint8Array;
  readonly ed25519: CryptoKey;
  readonly mlDsa: Uint8Array;
}
export async function halves(_seed: Uint8Array): Promise<readonly [Uint8Array, Uint8Array]> {
  return Promise.resolve([new Uint8Array(), new Uint8Array()]);
}
export async function keyFrom(_seed: Uint8Array): Promise<DeviceKey | null> {
  const pair = await crypto.subtle.generateKey({ name: "Ed25519" }, false, ["sign", "verify"]);
  return { public: new Uint8Array(1344), ed25519: pair.privateKey, mlDsa: new Uint8Array() };
}
export async function sign(_key: DeviceKey, _message: Uint8Array): Promise<Uint8Array | null> {
  return Promise.resolve(new Uint8Array(2484));
}
export async function verifies(_public: Uint8Array, _message: Uint8Array, _signature: Uint8Array): Promise<boolean> {
  return Promise.resolve(false);
}
