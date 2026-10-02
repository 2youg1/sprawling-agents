// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The key exchange both handshakes share (`crates/remote_access/
// Spec.lean` §8-4): an ephemeral X25519 key from WebCrypto and an
// ephemeral ML-KEM-768 key from `@noble/post-quantum`, the transcript
// both sides sign, and the two session keys HKDF expands from both
// secrets under that transcript. A man in the middle needs both secrets
// to reach either key.

import { Result } from "effect";

export interface Ephemeral {
  readonly x25519: CryptoKey;
  // FIPS 203's 2400-byte decapsulation key.
  readonly mlKem: Uint8Array<ArrayBuffer>;
}

export interface SessionKeys {
  readonly deviceToCity: Uint8Array<ArrayBuffer>;
  readonly cityToDevice: Uint8Array<ArrayBuffer>;
}

const X25519_BITS = 256;

const postQuantum = () => import("@noble/post-quantum/ml-kem.js");

// A fresh ephemeral pair and its public half, X25519 then the ML-KEM
// encapsulation key, as a hello carries them; null when this browser
// has no X25519.
export async function ephemeral(): Promise<{ readonly ephemeral: Ephemeral; readonly public: Uint8Array<ArrayBuffer> } | null> {
  const pair = await crypto.subtle.generateKey({ name: "X25519" }, false, ["deriveBits"]).catch(() => null);
  if (pair === null || !("privateKey" in pair)) return null;
  const xPublic = new Uint8Array(await crypto.subtle.exportKey("raw", pair.publicKey));
  const { ml_kem768 } = await postQuantum();
  const kem = ml_kem768.keygen();
  return {
    ephemeral: { x25519: pair.privateKey, mlKem: kem.secretKey },
    public: new Uint8Array([...xPublic, ...kem.publicKey]),
  };
}

// The handshake's record: SHA-256 of the label, what the device opened
// with, and the city's answer without its signature. SHA-256 rather than
// the city's BLAKE3 because WebCrypto has the one and not the other.
export async function record(label: Uint8Array<ArrayBuffer>, opening: Uint8Array<ArrayBuffer>, unsignedAnswer: Uint8Array<ArrayBuffer>): Promise<Uint8Array<ArrayBuffer>> {
  return new Uint8Array(await crypto.subtle.digest("SHA-256", new Uint8Array([...label, ...opening, ...unsignedAnswer])));
}

// What one side signs: the label, its role, the record. The role keeps
// one side's signature from ever standing in for the other's.
export function signed(label: Uint8Array<ArrayBuffer>, role: "city" | "device", transcript: Uint8Array<ArrayBuffer>): Uint8Array<ArrayBuffer> {
  return new Uint8Array([...label, ...new TextEncoder().encode(role), ...transcript]);
}

// The two session keys: HKDF-SHA256, the record as salt, the X25519
// secret then the ML-KEM secret as input.
export async function derive(transcript: Uint8Array<ArrayBuffer>, xSecret: Uint8Array<ArrayBuffer>, kemSecret: Uint8Array<ArrayBuffer>): Promise<SessionKeys> {
  const material = await crypto.subtle.importKey("raw", new Uint8Array([...xSecret, ...kemSecret]), "HKDF", false, [
    "deriveBits",
  ]);
  const expand = async (label: string) =>
    new Uint8Array(
      await crypto.subtle.deriveBits(
        { name: "HKDF", hash: "SHA-256", salt: transcript, info: new TextEncoder().encode(label) },
        material,
        256,
      ),
    );
  return { deviceToCity: await expand("device to city"), cityToDevice: await expand("city to device") };
}

// The session keys the device reaches from the city's answer; null when
// the city's X25519 key or ML-KEM ciphertext is not one this side can use.
export async function agreed(
  mine: Ephemeral,
  transcript: Uint8Array<ArrayBuffer>,
  xPeer: Uint8Array<ArrayBuffer>,
  ciphertext: Uint8Array<ArrayBuffer>,
): Promise<SessionKeys | null> {
  const xSecret = await crypto.subtle
    .importKey("raw", xPeer, { name: "X25519" }, false, [])
    .then((peer) => crypto.subtle.deriveBits({ name: "X25519", public: peer }, mine.x25519, X25519_BITS))
    .catch(() => null);
  const { ml_kem768 } = await postQuantum();
  const kemSecret = Result.getOrNull(Result.try(() => ml_kem768.decapsulate(ciphertext, mine.mlKem)));
  return xSecret === null || kemSecret === null ? null : derive(transcript, new Uint8Array(xSecret), kemSecret);
}
