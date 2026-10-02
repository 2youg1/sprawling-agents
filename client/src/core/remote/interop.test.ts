// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The page's half of the remote door against the bytes the Rust city
// wrote (`crates/remote_access/Spec.lean` §8-12): the key derivation,
// the session keys, the seal, a Rust signature, and one handshake whose
// first frame the page must open. The page also signs once, into
// `signature-browser.txt`, which the Rust test verifies with the same
// code that verifies its own; `GOLDEN_WRITE=1` rewrites that file only
// when the one in the tree no longer verifies here.

import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync, writeFileSync } from "node:fs";

import { derive, type Ephemeral } from "./agreement";
import { finish } from "./handshake";
import { halves, keyFrom, sign, verifies } from "./keys";
import { opener, payloadBytes, payloadOf, sealer } from "./seal";

const DIR = new URL("../../../../tools/fixtures/remote-handshake/", import.meta.url);

type Fields = ReadonlyMap<string, Uint8Array>;

function bytesOf(hex: string): Uint8Array {
  return Uint8Array.from(hex.match(/../g) ?? [], (pair) => Number.parseInt(pair, 16));
}

function hexOf(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

function read(name: string): Fields {
  const text = readFileSync(new URL(name, DIR), "utf8");
  return new Map(
    text
      .trimEnd()
      .split("\n")
      .map((line) => line.split(" "))
      .map(([field = "", value = ""]) => [field, bytesOf(value)] as const),
  );
}

function field(fields: Fields, name: string): Uint8Array {
  return fields.get(name) ?? new Uint8Array();
}

// The fixed X25519 private key of the fixture, in the one form WebCrypto
// imports a raw X25519 private key from: PKCS #8 (RFC 8410).
async function x25519From(raw: Uint8Array): Promise<CryptoKey> {
  const pkcs8 = new Uint8Array([...bytesOf("302e020100300506032b656e04220420"), ...raw]);
  return crypto.subtle.importKey("pkcs8", pkcs8, { name: "X25519" }, false, ["deriveBits"]);
}

describe("the key a seed derives", () => {
  test("splits into the two halves the Rust city derives", async () => {
    const keys = read("keys.txt");
    const [ed25519, mlDsa] = await halves(field(keys, "seed"));
    expect([hexOf(ed25519), hexOf(mlDsa)]).toEqual([
      hexOf(field(keys, "ed25519_seed")),
      hexOf(field(keys, "ml_dsa_44_seed")),
    ]);
  });

  test("has the public key the Rust city derives", async () => {
    const keys = read("keys.txt");
    const key = await keyFrom(field(keys, "seed"));
    expect(hexOf(key?.public ?? new Uint8Array())).toBe(hexOf(field(keys, "public")));
  });
});

describe("the session keys", () => {
  test("expand from the transcript and both secrets as the city's do", async () => {
    const vector = read("session-keys.txt");
    const keys = await derive(
      field(vector, "transcript"),
      field(vector, "x25519_secret"),
      field(vector, "ml_kem_secret"),
    );
    expect([hexOf(keys.deviceToCity), hexOf(keys.cityToDevice)]).toEqual([
      hexOf(field(vector, "device_to_city")),
      hexOf(field(vector, "city_to_device")),
    ]);
  });
});

describe("the seal", () => {
  test("seals a frame and then a lock to the city's bytes, and opens them back", async () => {
    const vector = read("seal.txt");
    const key = field(vector, "key");
    const city = sealer(key, "city_to_device");
    const sealed = [await city.seal(field(vector, "frame")), await city.seal(field(vector, "lock"))];
    const device = opener(key, "city_to_device");
    const opened = [
      await device.open(field(vector, "frame_sealed")),
      await device.open(field(vector, "lock_sealed")),
    ];
    expect({
      sealed: sealed.map((each) => hexOf(each ?? new Uint8Array())),
      opened: opened.map((each) => payloadOf(each ?? new Uint8Array())),
      spelled: hexOf(payloadBytes({ kind: "lock" })),
    }).toEqual({
      sealed: [hexOf(field(vector, "frame_sealed")), hexOf(field(vector, "lock_sealed"))],
      opened: [payloadOf(field(vector, "frame")), { kind: "lock" }],
      spelled: hexOf(field(vector, "lock")),
    });
  });

  test("opens nothing out of order or in the other direction", async () => {
    const vector = read("seal.txt");
    const key = field(vector, "key");
    expect([
      await opener(key, "city_to_device").open(field(vector, "lock_sealed")),
      await opener(key, "device_to_city").open(field(vector, "frame_sealed")),
    ]).toEqual([null, null]);
  });
});

describe("a signature", () => {
  test("the Rust city made verifies here, and fails with one byte changed", async () => {
    const vector = read("signature-rust.txt");
    const signature = field(vector, "signature");
    const changed = Uint8Array.from(signature, (byte, at) => (at === 100 ? byte ^ 1 : byte));
    expect([
      await verifies(field(vector, "public"), field(vector, "message"), signature),
      await verifies(field(vector, "public"), field(vector, "message"), changed),
    ]).toEqual([true, false]);
  });

  test("this page makes, over the same message, is the one in the tree", async () => {
    const keys = read("keys.txt");
    const message = field(read("signature-rust.txt"), "message");
    const key = await keyFrom(field(keys, "seed"));
    const kept = new URL("signature-browser.txt", DIR);
    const holds = async (): Promise<boolean> => {
      if (!existsSync(kept)) return false;
      const vector = read("signature-browser.txt");
      return (
        hexOf(field(vector, "public")) === hexOf(field(keys, "public")) &&
        hexOf(field(vector, "message")) === hexOf(message) &&
        (await verifies(field(vector, "public"), message, field(vector, "signature")))
      );
    };
    if (process.env["GOLDEN_WRITE"] === "1" && !(await holds()) && key !== null) {
      const signature = (await sign(key, message)) ?? new Uint8Array();
      const lines = [
        `public ${hexOf(key.public)}`,
        `message ${hexOf(message)}`,
        `signature ${hexOf(signature)}`,
      ];
      writeFileSync(kept, `${lines.join("\n")}\n`);
    }
    expect(await holds()).toBe(true);
  });
});

describe("the one-way fixture", () => {
  test("a device holding the fixed ephemeral keys opens the city's first frame", async () => {
    const fixture = read("fixture.txt");
    const ephemeral: Ephemeral = {
      x25519: await x25519From(field(fixture, "x25519_private")),
      mlKem: field(fixture, "ml_kem_private"),
    };
    const device = await keyFrom(new Uint8Array(32).fill(5));
    const finished =
      device === null
        ? "no key"
        : await finish(
            { hello: field(fixture, "hello"), ephemeral },
            field(fixture, "reply"),
            field(fixture, "city_public"),
            device,
          );
    const opened =
      typeof finished === "string" ? null : await finished.session.opener.open(field(fixture, "first_frame"));
    expect(hexOf(opened ?? new Uint8Array())).toBe(hexOf(field(fixture, "plaintext")));
  });
});
