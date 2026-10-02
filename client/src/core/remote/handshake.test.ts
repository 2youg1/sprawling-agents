// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The device's half of the pairing handshake (`crates/remote_access/
// Spec.lean` §8-6) against a city written here from the protocol's own
// words, because the Rust city's pairing reply is drawn from randomness
// no fixture can hold still. What the city opens is the claim the
// device sealed; what the device refuses, it refuses before the code
// leaves it.

import { describe, expect, test } from "bun:test";
import { ml_kem768 } from "@noble/post-quantum/ml-kem.js";

import { derive, record, signed } from "./agreement";
import { claim, pairHello, type DevicePairing } from "./handshake";
import type { Invitation } from "./invitation";
import { keyFrom, sign, verifies, type DeviceKey } from "./keys";
import { opener } from "./seal";

const PAIRING = new TextEncoder().encode("sprawling remote pairing v1");
const CODE = "abcdefghijklmnopqrstuvwxyz";

interface City {
  readonly key: DeviceKey;
  readonly fingerprint: Uint8Array<ArrayBuffer>;
}

async function city(): Promise<City> {
  const key = await keyFrom(new Uint8Array(32).fill(3));
  if (key === null) return { key: { public: new Uint8Array(), ed25519: await nothing(), mlDsa: new Uint8Array() }, fingerprint: new Uint8Array() };
  return { key, fingerprint: new Uint8Array(await crypto.subtle.digest("SHA-256", key.public)) };
}

async function nothing(): Promise<CryptoKey> {
  const pair = await crypto.subtle.generateKey({ name: "Ed25519" }, false, ["sign", "verify"]);
  return pair.privateKey;
}

// The city's reply to a pairing hello, and the keys it then holds.
async function reply(
  at: City,
  hello: Uint8Array<ArrayBuffer>,
): Promise<{ readonly reply: Uint8Array<ArrayBuffer>; readonly transcript: Uint8Array<ArrayBuffer>; readonly deviceToCity: Uint8Array<ArrayBuffer> }> {
  const devicePublic = hello.slice(0, 32);
  const encapsulationKey = hello.slice(32, 32 + 1184);
  const mine = await crypto.subtle.generateKey({ name: "X25519" }, true, ["deriveBits"]);
  const peer = await crypto.subtle.importKey("raw", devicePublic, { name: "X25519" }, false, []);
  const xSecret = new Uint8Array(await crypto.subtle.deriveBits({ name: "X25519", public: peer }, mine.privateKey, 256));
  const xPublic = new Uint8Array(await crypto.subtle.exportKey("raw", mine.publicKey));
  const { cipherText, sharedSecret } = ml_kem768.encapsulate(encapsulationKey);
  const unsigned = new Uint8Array([...at.key.public, ...xPublic, ...cipherText, ...new Uint8Array(32).fill(4)]);
  const transcript = await record(PAIRING, hello, unsigned);
  const signature = (await sign(at.key, signed(PAIRING, "city", transcript))) ?? new Uint8Array();
  const keys = await derive(transcript, xSecret, sharedSecret);
  return { reply: new Uint8Array([...unsigned, ...signature]), transcript, deviceToCity: keys.deviceToCity };
}

async function started(): Promise<DevicePairing> {
  const pairing = await pairHello(new Uint8Array(32).fill(2));
  expect(pairing).not.toBeNull();
  return pairing ?? { hello: new Uint8Array(), ephemeral: { x25519: await nothing(), mlKem: new Uint8Array() } };
}

describe("a pairing", () => {
  test("seals the code, the device's key and its proof for the city that was invited", async () => {
    const at = await city();
    const device = await keyFrom(new Uint8Array(32).fill(9));
    const pairing = await started();
    const answered = await reply(at, pairing.hello);
    const invitation: Invitation = { code: CODE, city: at.fingerprint };
    const claimed = device === null ? "no key" : await claim(pairing, answered.reply, invitation, device);
    const sealed = typeof claimed === "string" ? new Uint8Array() : claimed.sealed;
    const opened = (await opener(answered.deviceToCity, "device_to_city").open(sealed)) ?? new Uint8Array();
    const code = new TextDecoder().decode(opened.slice(0, 26));
    const devicePublic = opened.slice(26, 26 + 1344);
    const proof = opened.slice(26 + 1344);
    expect({
      code,
      key: devicePublic.join(),
      proven: await verifies(devicePublic, signed(PAIRING, "device", answered.transcript), proof),
      city: typeof claimed === "string" ? "" : claimed.city.join(),
    }).toEqual({ code: CODE, key: device?.public.join() ?? "", proven: true, city: at.key.public.join() });
  });

  test("stops before the claim when the city is not the one the invitation names", async () => {
    const at = await city();
    const device = await keyFrom(new Uint8Array(32).fill(9));
    const pairing = await started();
    const answered = await reply(at, pairing.hello);
    const elsewhere: Invitation = { code: CODE, city: new Uint8Array(32).fill(1) };
    expect(device === null ? "no key" : await claim(pairing, answered.reply, elsewhere, device)).toBe("fingerprint");
  });

  test("stops before the claim when the city's signature is not its own", async () => {
    const at = await city();
    const device = await keyFrom(new Uint8Array(32).fill(9));
    const pairing = await started();
    const answered = await reply(at, pairing.hello);
    const forged = Uint8Array.from(answered.reply, (byte, index) =>
      index === answered.reply.length - 1 ? byte ^ 1 : byte,
    );
    const invitation: Invitation = { code: CODE, city: at.fingerprint };
    expect(device === null ? "no key" : await claim(pairing, forged, invitation, device)).toBe("signature");
  });

  test("refuses a reply of the wrong length", async () => {
    const at = await city();
    const device = await keyFrom(new Uint8Array(32).fill(9));
    const pairing = await started();
    const invitation: Invitation = { code: CODE, city: at.fingerprint };
    expect(device === null ? "no key" : await claim(pairing, new Uint8Array(12), invitation, device)).toBe("length");
  });
});
