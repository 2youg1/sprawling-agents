// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export interface Ephemeral {
  readonly x25519: CryptoKey;
  readonly mlKem: Uint8Array;
}
export interface SessionKeys {
  readonly deviceToCity: Uint8Array;
  readonly cityToDevice: Uint8Array;
}
export async function record(_label: Uint8Array, _opening: Uint8Array, _unsigned: Uint8Array): Promise<Uint8Array> {
  return Promise.resolve(new Uint8Array(32));
}
export function signed(_label: Uint8Array, _role: "city" | "device", transcript: Uint8Array): Uint8Array {
  return transcript;
}
export async function derive(_t: Uint8Array, _x: Uint8Array, _k: Uint8Array): Promise<SessionKeys> {
  return Promise.resolve({ deviceToCity: new Uint8Array(32), cityToDevice: new Uint8Array(32) });
}
