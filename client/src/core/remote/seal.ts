// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Every frame of a session, sealed (`crates/remote_access/Spec.lean`
// §8-5): AES-256-GCM, the nonce a four-byte direction tag then an
// eight-byte big-endian count that never travels, because the socket
// delivers in order and each side knows the next number. A frame that
// will not open ends the session: no frame is worth skipping.
//
// The count is taken when `seal` or `open` is called, before anything
// is awaited, so two seals in flight never share a nonce.

import { Result } from "effect";

export type Direction = "device_to_city" | "city_to_device";

export interface Sealer {
  readonly seal: (bytes: Uint8Array<ArrayBuffer>) => Promise<Uint8Array<ArrayBuffer> | null>;
}

export interface Opener {
  // Null when the frame was changed, replayed, dropped or sent the other
  // way; the count does not move, and the caller ends the session.
  readonly open: (sealed: Uint8Array<ArrayBuffer>) => Promise<Uint8Array<ArrayBuffer> | null>;
}

// What a sealed frame of a session carries, told by its first byte:
// a wire text frame, or the lock (D13).
export type Payload = { readonly kind: "frame"; readonly text: string } | { readonly kind: "lock" };

const FRAME = 0;
const LOCK = 1;

const TAGS: Readonly<Record<Direction, string>> = { device_to_city: "d->c", city_to_device: "c->d" };

function nonce(direction: Direction, count: bigint): Uint8Array<ArrayBuffer> {
  const bytes = new Uint8Array(12);
  bytes.set(new TextEncoder().encode(TAGS[direction]));
  new DataView(bytes.buffer).setBigUint64(4, count);
  return bytes;
}

function aes(key: Uint8Array<ArrayBuffer>, use: KeyUsage): Promise<CryptoKey | null> {
  return crypto.subtle.importKey("raw", key, { name: "AES-GCM" }, false, [use]).catch(() => null);
}

export function sealer(key: Uint8Array<ArrayBuffer>, direction: Direction): Sealer {
  const loaded = aes(key, "encrypt");
  let next = 0n;
  return {
    async seal(bytes) {
      const iv = nonce(direction, next);
      next += 1n;
      const held = await loaded;
      if (held === null) return null;
      const sealed = await crypto.subtle.encrypt({ name: "AES-GCM", iv }, held, bytes).catch(() => null);
      return sealed === null ? null : new Uint8Array(sealed);
    },
  };
}

export function opener(key: Uint8Array<ArrayBuffer>, direction: Direction): Opener {
  const loaded = aes(key, "decrypt");
  let next = 0n;
  return {
    async open(sealed) {
      const count = next;
      const held = await loaded;
      if (held === null) return null;
      const opened = await crypto.subtle
        .decrypt({ name: "AES-GCM", iv: nonce(direction, count) }, held, sealed)
        .catch(() => null);
      if (opened === null) return null;
      next = count + 1n;
      return new Uint8Array(opened);
    },
  };
}

export function payloadBytes(payload: Payload): Uint8Array<ArrayBuffer> {
  switch (payload.kind) {
    case "frame":
      return new Uint8Array([FRAME, ...new TextEncoder().encode(payload.text)]);
    case "lock":
      return Uint8Array.of(LOCK);
  }
}

// Null for an empty payload, an unknown first byte, bytes after a lock,
// or a frame that is not UTF-8: the city's `E_WIRE_MISMATCH` cases.
export function payloadOf(bytes: Uint8Array<ArrayBuffer>): Payload | null {
  const [first, ...rest] = bytes;
  if (first === LOCK) return rest.length === 0 ? { kind: "lock" } : null;
  if (first !== FRAME) return null;
  const text = new TextDecoder("utf-8", { fatal: true });
  const decoded = Result.getOrNull(Result.try(() => text.decode(bytes.slice(1))));
  return decoded === null ? null : { kind: "frame", text: decoded };
}
