// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { agreed, ephemeral, record, signed, type Ephemeral, type SessionKeys } from "./agreement";
import type { Invitation } from "./invitation";
import { PUBLIC_BYTES, sign, verifies, type DeviceKey } from "./keys";
import { opener, sealer, type Opener, type Sealer } from "./seal";

export type Refused = "length" | "fingerprint" | "signature" | "agreement" | "key";

export interface Session {
  readonly sealer: Sealer;
  readonly opener: Opener;
}

export interface DevicePairing {
  readonly hello: Uint8Array<ArrayBuffer>;
  readonly ephemeral: Ephemeral;
}

export type DeviceWaiting = DevicePairing;

export interface Claimed {

  readonly sealed: Uint8Array<ArrayBuffer>;

  readonly city: Uint8Array<ArrayBuffer>;
  readonly session: Session;
}

export interface Finished {
  readonly finish: Uint8Array<ArrayBuffer>;
  readonly session: Session;
}

const PROTOCOL = new TextEncoder().encode("sprawling remote handshake v1");
const PAIRING = new TextEncoder().encode("sprawling remote pairing v1");

const DEVICE_ID_BYTES = 16;
const X25519_BYTES = 32;
const ML_KEM_CIPHERTEXT_BYTES = 1088;
const NONCE_BYTES = 32;
const SIGNATURE_BYTES = 64 + 2420;
const UNSIGNED_REPLY_BYTES = X25519_BYTES + ML_KEM_CIPHERTEXT_BYTES + NONCE_BYTES;
const UNSIGNED_PAIR_REPLY_BYTES = PUBLIC_BYTES + UNSIGNED_REPLY_BYTES;

function sessionOf(keys: SessionKeys): Session {
  return {
    sealer: sealer(keys.deviceToCity, "device_to_city"),
    opener: opener(keys.cityToDevice, "city_to_device"),
  };
}

async function opened(
  opening: Uint8Array<ArrayBuffer>,
  nonce: Uint8Array<ArrayBuffer>,
): Promise<DevicePairing | null> {
  const fresh = await ephemeral();
  return fresh === null
    ? null
    : {
        hello: new Uint8Array([...opening, ...fresh.public, ...nonce]),
        ephemeral: fresh.ephemeral,
      };
}

export function pairHello(nonce: Uint8Array<ArrayBuffer>): Promise<DevicePairing | null> {
  return opened(new Uint8Array(), nonce);
}

export function sessionHello(
  device: Uint8Array<ArrayBuffer>,
  nonce: Uint8Array<ArrayBuffer>,
): Promise<DeviceWaiting | null> {
  return device.length === DEVICE_ID_BYTES ? opened(device, nonce) : Promise.resolve(null);
}

export async function claim(
  pairing: DevicePairing,
  reply: Uint8Array<ArrayBuffer>,
  invitation: Invitation,
  key: DeviceKey,
): Promise<Claimed | Refused> {
  if (reply.length !== UNSIGNED_PAIR_REPLY_BYTES + SIGNATURE_BYTES) return "length";
  const presented = reply.slice(0, PUBLIC_BYTES);
  const fingerprint = new Uint8Array(await crypto.subtle.digest("SHA-256", presented));
  if (fingerprint.join() !== invitation.city.join()) return "fingerprint";
  const answer = reply.slice(PUBLIC_BYTES);
  const transcript = await record(
    PAIRING,
    pairing.hello,
    reply.slice(0, UNSIGNED_PAIR_REPLY_BYTES),
  );
  const agreement = await answered(pairing, PAIRING, transcript, answer, presented);
  if (typeof agreement === "string") return agreement;
  const proof = await sign(key, signed(PAIRING, "device", transcript));
  if (proof === null) return "key";
  const session = sessionOf(agreement);
  const sealed = await session.sealer.seal(
    new Uint8Array([...new TextEncoder().encode(invitation.code), ...key.public, ...proof]),
  );
  return sealed === null ? "key" : { sealed, city: presented, session };
}

export async function finish(
  waiting: DeviceWaiting,
  reply: Uint8Array<ArrayBuffer>,
  city: Uint8Array<ArrayBuffer>,
  key: DeviceKey,
): Promise<Finished | Refused> {
  if (reply.length !== UNSIGNED_REPLY_BYTES + SIGNATURE_BYTES) return "length";
  const transcript = await record(PROTOCOL, waiting.hello, reply.slice(0, UNSIGNED_REPLY_BYTES));
  const agreement = await answered(waiting, PROTOCOL, transcript, reply, city);
  if (typeof agreement === "string") return agreement;
  const signature = await sign(key, signed(PROTOCOL, "device", transcript));
  return signature === null ? "key" : { finish: signature, session: sessionOf(agreement) };
}

async function answered(
  mine: DevicePairing,
  label: Uint8Array<ArrayBuffer>,
  transcript: Uint8Array<ArrayBuffer>,
  answer: Uint8Array<ArrayBuffer>,
  city: Uint8Array<ArrayBuffer>,
): Promise<SessionKeys | Refused> {
  const xPeer = answer.slice(0, X25519_BYTES);
  const ciphertext = answer.slice(X25519_BYTES, X25519_BYTES + ML_KEM_CIPHERTEXT_BYTES);
  const signature = answer.slice(UNSIGNED_REPLY_BYTES);
  if (!(await verifies(city, signed(label, "city", transcript), signature))) return "signature";
  return (await agreed(mine.ephemeral, transcript, xPeer, ciphertext)) ?? "agreement";
}
