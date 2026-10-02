// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export type Direction = "device_to_city" | "city_to_device";
export interface Sealer {
  readonly seal: (bytes: Uint8Array) => Promise<Uint8Array | null>;
}
export interface Opener {
  readonly open: (bytes: Uint8Array) => Promise<Uint8Array | null>;
}
export type Payload = { readonly kind: "frame"; readonly text: string } | { readonly kind: "lock" };
export function sealer(_key: Uint8Array, _direction: Direction): Sealer {
  return { seal: async () => Promise.resolve(null) };
}
export function opener(_key: Uint8Array, _direction: Direction): Opener {
  return { open: async () => Promise.resolve(null) };
}
export function payloadBytes(_payload: Payload): Uint8Array {
  return new Uint8Array();
}
export function payloadOf(_bytes: Uint8Array): Payload | null {
  return null;
}
