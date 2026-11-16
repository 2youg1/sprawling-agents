// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Result } from "effect";

export interface Ephemeral {
  readonly x25519: CryptoKey;

  readonly mlKem: Uint8Array<ArrayBuffer>;
}

export interface SessionKeys {
  readonly deviceToCity: Uint8Array<ArrayBuffer>;
  readonly cityToDevice: Uint8Array<ArrayBuffer>;
}

const X25519_BITS = 256;

const postQuantum = () => import("@noble/post-quantum/ml-kem.js");

export async function ephemeral(): Promise<{
  readonly ephemeral: Ephemeral;
  readonly public: Uint8Array<ArrayBuffer>;
} | null> {
  const pair = await crypto.subtle
    .generateKey({ name: "X25519" }, false, ["deriveBits"])
    .catch(() => null);
  if (pair === null || !("privateKey" in pair)) return null;
  const xPublic = new Uint8Array(await crypto.subtle.exportKey("raw", pair.publicKey));
  const { ml_kem768 } = await postQuantum();
  const kem = ml_kem768.keygen();
  return {
    ephemeral: { x25519: pair.privateKey, mlKem: kem.secretKey },
    public: new Uint8Array([...xPublic, ...kem.publicKey]),
  };
}

export async function record(
  label: Uint8Array<ArrayBuffer>,
  opening: Uint8Array<ArrayBuffer>,
  unsignedAnswer: Uint8Array<ArrayBuffer>,
): Promise<Uint8Array<ArrayBuffer>> {
  return new Uint8Array(
    await crypto.subtle.digest(
      "SHA-256",
      new Uint8Array([...label, ...opening, ...unsignedAnswer]),
    ),
  );
}

export function signed(
  label: Uint8Array<ArrayBuffer>,
  role: "city" | "device",
  transcript: Uint8Array<ArrayBuffer>,
): Uint8Array<ArrayBuffer> {
  return new Uint8Array([...label, ...new TextEncoder().encode(role), ...transcript]);
}

export async function derive(
  transcript: Uint8Array<ArrayBuffer>,
  xSecret: Uint8Array<ArrayBuffer>,
  kemSecret: Uint8Array<ArrayBuffer>,
): Promise<SessionKeys> {
  const material = await crypto.subtle.importKey(
    "raw",
    new Uint8Array([...xSecret, ...kemSecret]),
    "HKDF",
    false,
    ["deriveBits"],
  );
  const expand = async (label: string) =>
    new Uint8Array(
      await crypto.subtle.deriveBits(
        { name: "HKDF", hash: "SHA-256", salt: transcript, info: new TextEncoder().encode(label) },
        material,
        256,
      ),
    );
  return {
    deviceToCity: await expand("device to city"),
    cityToDevice: await expand("city to device"),
  };
}

export async function agreed(
  mine: Ephemeral,
  transcript: Uint8Array<ArrayBuffer>,
  xPeer: Uint8Array<ArrayBuffer>,
  ciphertext: Uint8Array<ArrayBuffer>,
): Promise<SessionKeys | null> {
  const xSecret = await crypto.subtle
    .importKey("raw", xPeer, { name: "X25519" }, false, [])
    .then((peer) =>
      crypto.subtle.deriveBits({ name: "X25519", public: peer }, mine.x25519, X25519_BITS),
    )
    .catch(() => null);
  const { ml_kem768 } = await postQuantum();
  const kemSecret = Result.getOrNull(
    Result.try(() => ml_kem768.decapsulate(ciphertext, mine.mlKem)),
  );
  return xSecret === null || kemSecret === null
    ? null
    : derive(transcript, new Uint8Array(xSecret), kemSecret);
}
