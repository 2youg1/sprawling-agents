// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { Option } from "effect";

import { Address } from "./address";
import { RunId } from "./run_id";

describe("address", () => {
  test("accepts canonical relative paths", () => {
    for (const ok of [
      "a",
      "a/b",
      "docs/notes.md",
      "role@building.1/JOB.md",
      ".sprawling/ledger",
    ]) {
      expect<string | null>(Option.getOrNull(Address.option(ok)), ok).toBe(ok);
    }
  });

  test("rejects every banned form", () => {
    for (const bad of [
      "",
      "/abs",
      "a//b",
      "a/",
      "/",
      "..",
      "a/../b",
      ".",
      "a/./b",
      "a\\b",
      "C:/x",
      "a/b:stream",
      "a\u0000b",
      "a\tb",
    ]) {
      expect(Address.option(bad), JSON.stringify(bad)).toEqual(Option.none());
    }
  });
});

describe("run id", () => {
  test("reads a hyphenated uuid and a bare one, and writes the hyphenated form", () => {
    const hyphenated = "07070707-0707-0707-0707-070707070707";
    const read = (raw: string): string | null => Option.getOrNull(RunId.option(raw));
    expect(read(hyphenated)).toBe(hyphenated);
    expect(read("07070707070707070707070707070707")).toBe(hyphenated);
    expect(read("07070707-0707-0707-0707-07070707070A")).toBe(
      "07070707-0707-0707-0707-07070707070a",
    );
  });

  test("refuses what is not a uuid", () => {
    for (const bad of ["", "not-a-run", "0707070707070707070707070707070", "0707070707070707070707070707070g"]) {
      expect(RunId.option(bad), bad).toEqual(Option.none());
    }
  });
});
