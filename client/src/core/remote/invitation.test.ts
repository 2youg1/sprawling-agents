// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { canonical, decode, encode } from "./base32";
import { invitationIn, isInvitation } from "./invitation";

// A fingerprint as the city's console prints it: 32 bytes, 52 symbols.
const CITY = encode(new Uint8Array(32).map((_, at) => at));

describe("base32", () => {
  test("spells RFC 4648's vectors in lower case, without padding, and reads them back", () => {
    const words = ["", "f", "fo", "foo", "foob", "fooba", "foobar"];
    const spelled = words.map((word) => encode(new TextEncoder().encode(word)));
    expect({
      spelled,
      read: spelled.map((text) => new TextDecoder().decode(decode(text) ?? new Uint8Array([33]))),
    }).toEqual({
      spelled: ["", "my", "mzxq", "mzxw6", "mzxw6yq", "mzxw6ytb", "mzxw6ytboi"],
      read: words,
    });
  });

  test("reads a code a person copied by hand", () => {
    expect([canonical(" ABCDE-fghij klmno "), decode("mzxw6yt!")]).toEqual(["abcdefghijklmno", null]);
  });
});

describe("an invitation", () => {
  test("is read from the fragment the console's link carries", () => {
    const hash = `#pair=abcdefghijklmnopqrstuvwxyz&city=${CITY}`;
    expect({ asked: isInvitation(hash), read: invitationIn(hash) }).toEqual({
      asked: true,
      read: { code: "abcdefghijklmnopqrstuvwxyz", city: new Uint8Array(32).map((_, at) => at) },
    });
  });

  test("takes the code the way a person typed it", () => {
    const hash = `#pair=ABCDE-FGHIJ-KLMNO-PQRST-UVWXY-Z&city=${CITY.toUpperCase()}`;
    expect(invitationIn(hash)?.code).toBe("abcdefghijklmnopqrstuvwxyz");
  });

  test("is nothing when a part is short, missing or not the one spelling of its bytes", () => {
    // The last symbol carries one bit in its high end, so `b` (a low
    // bit set) spells no 32 bytes at all.
    expect([
      invitationIn(`#pair=abcde&city=${CITY}`),
      invitationIn(`#pair=abcdefghijklmnopqrstuvwxyz`),
      invitationIn(`#pair=abcdefghijklmnopqrstuvwxyz&city=${CITY.slice(0, -1)}b`),
      invitationIn(`#pair=abcdefghijklmnopqrstuvwx1z&city=${CITY}`),
    ]).toEqual([null, null, null, null]);
  });

  test("is asked for only by a fragment that carries the pairing key", () => {
    expect([isInvitation("#pair=x"), isInvitation("#/setup/remote"), isInvitation("")]).toEqual([true, false, false]);
  });
});
