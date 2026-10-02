// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { B3Hash, Locator } from "../wire";
import type { BytesAnswer } from "../wire";
import { DRAWN_BYTES_MAX, fetchedOf, fetching, joinedBytes, nextBytes, storedObject } from "./document_bytes";

const V = B3Hash.make("a".repeat(64));
const W = B3Hash.make("b".repeat(64));

function window(version: B3Hash, start: number, bytes: readonly number[], size: number): BytesAnswer {
  return {
    version,
    span: { start, end: start + bytes.length },
    size,
    base64: btoa(String.fromCharCode(...bytes)),
  };
}

describe("fetching a stored object", () => {
  test("asks from where the bytes reached, and is whole once they meet the end", () => {
    const start = fetching(V);
    expect(nextBytes(start)).toEqual({ start: 0, end: Number.MAX_SAFE_INTEGER });
    const first = joinedBytes(start, window(V, 0, [37, 80, 0], 5));
    expect(fetchedOf(first)).toEqual({ kind: "fetching", through: 3, size: 5 });
    expect(nextBytes(first)).toEqual({ start: 3, end: 5 });
    const both = joinedBytes(first, window(V, 3, [255, 10], 5));
    expect(nextBytes(both)).toBeNull();
    expect(fetchedOf(both)).toEqual({ kind: "whole", bytes: new Uint8Array([37, 80, 0, 255, 10]) });
  });

  test("an empty object is whole after one answer", () => {
    const empty = joinedBytes(fetching(V), window(V, 0, [], 0));
    expect(fetchedOf(empty)).toEqual({ kind: "whole", bytes: new Uint8Array([]) });
    expect(nextBytes(empty)).toBeNull();
  });

  test("a window of another version, out of order, or not base64 changes nothing", () => {
    const first = joinedBytes(fetching(V), window(V, 0, [1, 2], 4));
    expect(joinedBytes(first, window(W, 2, [3, 4], 4))).toEqual(first);
    expect(joinedBytes(first, window(V, 0, [1, 2], 4))).toEqual(first);
    expect(joinedBytes(first, { version: V, span: { start: 2, end: 4 }, size: 4, base64: "%%" })).toEqual(first);
  });

  test("an object past what the page draws is not fetched past its first answer", () => {
    const large = joinedBytes(fetching(V), window(V, 0, [1], DRAWN_BYTES_MAX + 1));
    expect(fetchedOf(large)).toEqual({ kind: "too_large", size: DRAWN_BYTES_MAX + 1 });
    expect(nextBytes(large)).toBeNull();
  });
});

describe("storedObject", () => {
  test("reads the whole object a cas locator names, and nothing else", () => {
    expect(storedObject(Locator.make(`cas:b3-${"c3".repeat(32)}`))).toBe(B3Hash.make("c3".repeat(32)));
    expect(storedObject(Locator.make(`cas:b3-${"c3".repeat(32)}#bytes=0-9`))).toBeNull();
    expect(storedObject(Locator.make(`file:shop/a.md@${"0".repeat(40)}`))).toBeNull();
  });
});
