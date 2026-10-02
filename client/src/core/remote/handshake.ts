// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Ephemeral } from "./agreement";
import type { Invitation } from "./invitation";
import type { DeviceKey } from "./keys";
import { opener, sealer, type Opener, type Sealer } from "./seal";

export type Refused = "length" | "fingerprint" | "signature" | "agreement";
export interface Session {
  readonly sealer: Sealer;
  readonly opener: Opener;
}
export interface DevicePairing {
  readonly hello: Uint8Array;
  readonly ephemeral: Ephemeral;
}
export interface Claimed {
  readonly sealed: Uint8Array;
  readonly city: Uint8Array;
  readonly session: Session;
}
export type DeviceWaiting = DevicePairing;
export interface Finished {
  readonly finish: Uint8Array;
  readonly session: Session;
}
export async function pairHello(_nonce: Uint8Array): Promise<DevicePairing | null> {
  const pair = await crypto.subtle.generateKey({ name: "X25519" }, true, ["deriveBits"]);
  const { ml_kem768 } = await import("@noble/post-quantum/ml-kem.js");
  const kem = ml_kem768.keygen();
  const x = new Uint8Array(await crypto.subtle.exportKey("raw", pair.publicKey));
  return { hello: new Uint8Array([...x, ...kem.publicKey, ...new Uint8Array(32)]), ephemeral: { x25519: pair.privateKey, mlKem: kem.secretKey } };
}
export async function claim(_p: DevicePairing, _r: Uint8Array, _i: Invitation, _k: DeviceKey): Promise<Claimed | Refused> {
  return Promise.resolve({ sealed: new Uint8Array(), city: new Uint8Array(), session: { sealer: sealer(new Uint8Array(), "device_to_city"), opener: opener(new Uint8Array(), "city_to_device") } });
}
export async function finish(_w: DeviceWaiting, _r: Uint8Array, _c: Uint8Array, _k: DeviceKey): Promise<Finished | Refused> {
  return Promise.resolve({ finish: new Uint8Array(), session: { sealer: sealer(new Uint8Array(), "device_to_city"), opener: opener(new Uint8Array(), "city_to_device") } });
}
