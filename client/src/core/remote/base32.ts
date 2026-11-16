// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const ALPHABET = "abcdefghijklmnopqrstuvwxyz234567";
const SYMBOL_BITS = 5;

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

export function canonical(typed: string): string {
  return typed.replace(/[\s-]/g, "").toLowerCase();
}
