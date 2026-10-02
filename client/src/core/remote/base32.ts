// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The one spelling a person meets for every string of bytes the remote
// door shows: the pairing code, the city's fingerprint and the device's
// seed. RFC 4648 base32 in lower case without padding, the alphabet of
// `remote_access::pairing`, so a code the console printed and a code
// this page reads are the same characters (`crates/remote_access/
// Spec.lean` §8-2, D18).

const ALPHABET = "abcdefghijklmnopqrstuvwxyz234567";
const SYMBOL_BITS = 5;

// Every five bits one symbol; the last symbol carries the leftover bits
// in its high end.
export function encode(bytes: Uint8Array<ArrayBuffer>): string {
  let out = "";
  let buffer = 0;
  let held = 0;
  for (const byte of bytes) {
    buffer = (buffer << 8) | byte;
    held += 8;
    while (held >= SYMBOL_BITS) {
      held -= SYMBOL_BITS;
      out += ALPHABET.charAt((buffer >> held) & 31);
    }
    buffer &= (1 << held) - 1;
  }
  return held > 0 ? out + ALPHABET.charAt((buffer << (SYMBOL_BITS - held)) & 31) : out;
}

// The bytes a canonical text spells, or null for a symbol outside the
// alphabet. Bits left over at the end are dropped, as the city drops
// them; whether they were zero is the reader's question (`invitation.ts`
// asks it of a fingerprint).
export function decode(text: string): Uint8Array<ArrayBuffer> | null {
  const out: number[] = [];
  let buffer = 0;
  let held = 0;
  for (const symbol of text) {
    const value = ALPHABET.indexOf(symbol);
    if (value < 0) return null;
    buffer = ((buffer << SYMBOL_BITS) | value) & 0xfff;
    held += SYMBOL_BITS;
    if (held >= 8) {
      held -= 8;
      out.push((buffer >> held) & 0xff);
    }
  }
  return Uint8Array.from(out);
}

// What a person copied by hand, as the city reads it: no white space,
// no hyphens, lower case.
export function canonical(typed: string): string {
  return typed.replace(/[\s-]/g, "").toLowerCase();
}
