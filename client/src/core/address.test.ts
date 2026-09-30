// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The address grammar is `kernel::Address::parse`, stated as a pattern
// in `kernel::schema` and carried into `wire.ts` by `cargo xtask
// wire-ts`. This client no longer spells it; what it holds is that the
// generated schema gives every spelling in `tools/fixtures/address.jsonl` the
// verdict the Rust constructor gives it. That file is the one table both
// test suites read, so a case added there is judged on both sides.

import { describe, expect, test } from "bun:test";
import { Option, Schema } from "effect";

import { Address, RunId } from "../wire";
import { readRunId } from "./run_id";

const read = Schema.decodeOption(Address);

const Spelling = Schema.parseJson(
  Schema.Struct({
    address: Schema.String,
    verdict: Schema.Literal("accepted", "refused"),
  }),
);

const table = (await Bun.file(new URL("../../../tools/fixtures/address.jsonl", import.meta.url)).text())
  .split("\n")
  .filter((line) => line.length > 0)
  .map((line) => Schema.decodeUnknownSync(Spelling)(line));

describe("address", () => {
  test("the shared table holds both verdicts", () => {
    expect(table.some((row) => row.verdict === "accepted")).toBe(true);
    expect(table.some((row) => row.verdict === "refused")).toBe(true);
  });

  test("every spelling in the shared table gets its verdict", () => {
    for (const row of table) {
      const expected = row.verdict === "accepted" ? row.address : null;
      expect<string | null>(Option.getOrNull(read(row.address)), JSON.stringify(row.address)).toBe(expected);
    }
  });
});

describe("run id", () => {
  test("reads the one spelling the city writes", () => {
    const hyphenated = "07070707-0707-0707-0707-070707070707";
    expect(Option.getOrNull(readRunId(hyphenated))).toEqual(RunId.make(hyphenated));
  });

  // `uuid::Uuid::parse_str` reads every one of these; `kernel::RunId::
  // parse` reads none of them, because the city writes the hyphenated
  // lower-case form and nothing else. A client grammar of its own
  // accepted the first two, so a link this city would refuse opened a
  // page here.
  test("refuses every spelling that is not the one the city writes", () => {
    for (const bad of [
      "",
      "not-a-run",
      "0707070707070707070707070707070",
      "07070707070707070707070707070707",
      "07070707-0707-0707-0707-07070707070A",
      "{07070707-0707-0707-0707-070707070707}",
      "urn:uuid:07070707-0707-0707-0707-070707070707",
    ]) {
      expect(Option.getOrNull(readRunId(bad)), bad).toBeNull();
    }
  });
});
