// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The device's own key: a hybrid of Ed25519 and ML-DSA-44, both halves
// derived from one 32-byte seed exactly as the city derives its own
// (`crates/remote_access/Spec.lean` §8-3), so a signature holds only
// when both halves hold. The Ed25519 half is a WebCrypto key this page
// can sign with and no script can export (D1); the ML-DSA half has no
// WebCrypto form, so its private bytes are held as bytes (D21).
//
// `@noble/post-quantum` is fetched here, by dynamic import, the first
// time a key is made or checked, so a page that never pairs never
// downloads it (client-SPEC 4-57).

import { Result } from "effect";

export const SEED_BYTES = 32;
const ED25519_PUBLIC_BYTES = 32;
const ED25519_SIGNATURE_BYTES = 64;
export const PUBLIC_BYTES = ED25519_PUBLIC_BYTES + 1312;
export const SIGNATURE_BYTES = ED25519_SIGNATURE_BYTES + 2420;

const DERIVATION_SALT = new TextEncoder().encode("sprawling remote key v1");
// RFC 8410's PKCS #8 wrapping of a raw Ed25519 private key, the one form
// WebCrypto imports one from.
const ED25519_PKCS8 = Uint8Array.of(0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20);

export interface DeviceKey {
  // Ed25519 then ML-DSA-44, as the city stores a device's key.
  readonly public: Uint8Array<ArrayBuffer>;
  readonly ed25519: CryptoKey;
  readonly mlDsa: Uint8Array<ArrayBuffer>;
}

const postQuantum = () => import("@noble/post-quantum/ml-dsa.js");

// The two 32-byte sub-seeds: HKDF-SHA256 under the derivation salt, one
// label per half, so the halves share no key material.
export async function halves(seed: Uint8Array<ArrayBuffer>): Promise<readonly [Uint8Array<ArrayBuffer>, Uint8Array<ArrayBuffer>]> {
  const material = await crypto.subtle.importKey("raw", seed, "HKDF", false, ["deriveBits"]);
  const half = async (label: string) =>
    new Uint8Array(
      await crypto.subtle.deriveBits(
        { name: "HKDF", hash: "SHA-256", salt: DERIVATION_SALT, info: new TextEncoder().encode(label) },
        material,
        256,
      ),
    );
  return [await half("ed25519"), await half("ml-dsa-44")];
}

// The key a seed makes; null when this browser has no Ed25519 in its
// WebCrypto, which leaves it unable to hold a device key at all.
export async function keyFrom(seed: Uint8Array<ArrayBuffer>): Promise<DeviceKey | null> {
  const [edSeed, mlSeed] = await halves(seed);
  const pkcs8 = new Uint8Array([...ED25519_PKCS8, ...edSeed]);
  const shown = await crypto.subtle
    .importKey("pkcs8", pkcs8, { name: "Ed25519" }, true, ["sign"])
    .then((key) => crypto.subtle.exportKey("jwk", key))
    .catch(() => null);
  const kept = await crypto.subtle
    .importKey("pkcs8", pkcs8, { name: "Ed25519" }, false, ["sign"])
    .catch(() => null);
  const edPublic = shown?.x === undefined ? null : fromBase64Url(shown.x);
  if (kept === null || edPublic === null) return null;
  const { ml_dsa44 } = await postQuantum();
  const ml = ml_dsa44.keygen(mlSeed);
  return { public: new Uint8Array([...edPublic, ...ml.publicKey]), ed25519: kept, mlDsa: ml.secretKey };
}

// Both halves' signatures, Ed25519 first; null when either refused.
export async function sign(key: DeviceKey, message: Uint8Array<ArrayBuffer>): Promise<Uint8Array<ArrayBuffer> | null> {
  const ed = await crypto.subtle.sign({ name: "Ed25519" }, key.ed25519, message).catch(() => null);
  const { ml_dsa44 } = await postQuantum();
  const ml = Result.getOrNull(Result.try(() => ml_dsa44.sign(message, key.mlDsa)));
  return ed === null || ml === null ? null : new Uint8Array([...new Uint8Array(ed), ...ml]);
}

// Whether both halves hold. One answer, never which half failed.
export async function verifies(
  publicKey: Uint8Array<ArrayBuffer>,
  message: Uint8Array<ArrayBuffer>,
  signature: Uint8Array<ArrayBuffer>,
): Promise<boolean> {
  if (publicKey.length !== PUBLIC_BYTES || signature.length !== SIGNATURE_BYTES) return false;
  const edHolds = await crypto.subtle
    .importKey("raw", publicKey.slice(0, ED25519_PUBLIC_BYTES), { name: "Ed25519" }, false, ["verify"])
    .then((key) =>
      crypto.subtle.verify({ name: "Ed25519" }, key, signature.slice(0, ED25519_SIGNATURE_BYTES), message),
    )
    .catch(() => false);
  const { ml_dsa44 } = await postQuantum();
  const mlHolds = Result.getOrElse(
    Result.try(() =>
      ml_dsa44.verify(signature.slice(ED25519_SIGNATURE_BYTES), message, publicKey.slice(ED25519_PUBLIC_BYTES)),
    ),
    () => false,
  );
  return edHolds && mlHolds;
}

function fromBase64Url(text: string): Uint8Array<ArrayBuffer> | null {
  const padded = text.replace(/-/g, "+").replace(/_/g, "/");
  return Result.getOrNull(Result.try(() => Uint8Array.from(atob(padded), (char) => char.charCodeAt(0))));
}
