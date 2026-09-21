// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The address grammar is `kernel::Address::parse`, stated as a pattern
// in `kernel::schema` and carried into `wire.ts` by `cargo xtask
// wire-ts`. This client no longer spells it; what it holds is that the
// generated schema still refuses what the Rust constructor refuses.

import { describe, expect, test } from "bun:test";
import { Option, Schema } from "effect";

import { Address } from "../wire";
import { RunId } from "./run_id";

const read = Schema.decodeOption(Address);

describe("address", () => {
  test("accepts canonical relative paths", () => {
    for (const ok of [
      "a",
      "a/b",
      "docs/notes.md",
      "role@building.1/JOB.md",
      ".sprawling/ledger",
    ]) {
      expect<string | null>(Option.getOrNull(read(ok)), ok).toBe(ok);
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
      "a /b",
      "trailing.",
    ]) {
      expect(read(bad), JSON.stringify(bad)).toEqual(Option.none());
    }
  });
});

describe("run id", () => {
  test("reads a hyphenated uuid and a bare one, and writes the hyphenated form", () => {
    const hyphenated = "07070707-0707-0707-0707-070707070707";
    const readRun = (raw: string): string | null => Option.getOrNull(RunId.option(raw));
    expect(readRun(hyphenated)).toBe(hyphenated);
    expect(readRun("07070707070707070707070707070707")).toBe(hyphenated);
    expect(readRun("07070707-0707-0707-0707-07070707070A")).toBe(
      "07070707-0707-0707-0707-07070707070a",
    );
  });

  test("refuses what is not a uuid", () => {
    for (const bad of ["", "not-a-run", "0707070707070707070707070707070", "0707070707070707070707070707070g"]) {
      expect(RunId.option(bad), bad).toEqual(Option.none());
    }
  });
});
