// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { B3Hash, RunId, Seq, TimeMs } from "../../wire";
import type { EventRecord, LogLine } from "../../wire";
import { WIDE, entriesOf } from "./timeline";

const ONE = RunId.make("00000000-0000-0000-0000-000000000001");
const TWO = RunId.make("00000000-0000-0000-0000-000000000002");

function record(seq: number, run = ONE): EventRecord {
  return {
    run,
    seq: Seq.make(seq),
    kind: "run_started",
    t: TimeMs.make(seq * 1_000),
    who: "hall/mayor",
    prev: B3Hash.make("0".repeat(64)),
    v: 1,
    data: {},
  };
}

function line(seq: number, run: RunId | null = null): LogLine {
  return { seq: Seq.make(seq), level: "effect", module: "runtime", line: `at ${String(seq)}`, run, t: null };
}

const keys = (entries: readonly { readonly key: string }[]): readonly string[] => entries.map((each) => each.key);

describe("the record's one timeline", () => {
  test("a line stands above the record its ledger position follows, newest first", () => {
    const page = { records: [record(4), record(5)], head: true };
    expect(keys(entriesOf(page, [line(4), line(6)], "every", WIDE))).toEqual(["l6-1", "r5", "l4-0", "r4"]);
  });

  test("an earlier page shows only the lines written while its records were", () => {
    const page = { records: [record(4), record(5)], head: false };
    expect(keys(entriesOf(page, [line(3), line(5), line(9)], "every", WIDE))).toEqual(["l5-0", "r5", "r4"]);
  });

  test("the log read alone is its own window, whatever page the ledger is on", () => {
    const page = { records: [record(4)], head: false };
    expect(keys(entriesOf(page, [line(1), line(9)], "log", WIDE))).toEqual(["l9-1", "l1-0"]);
  });

  test("a run narrows both sources, and a level leaves no ledger record", () => {
    const page = { records: [record(4, ONE), record(5, TWO)], head: true };
    const lines = [line(5, TWO), line(6, ONE)];
    expect(keys(entriesOf(page, lines, "every", { ...WIDE, run: TWO }))).toEqual(["l5-0", "r5"]);
    expect(keys(entriesOf(page, lines, "every", { ...WIDE, level: "effect" }))).toEqual(["l6-1", "l5-0"]);
  });
});
