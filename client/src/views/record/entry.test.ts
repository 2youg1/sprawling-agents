// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The timeline's rows and narrowing choices, driven the way their seats
// drive them and with no look: what a press hands on, and what each row
// says it is.

import { describe, expect, test } from "bun:test";

import { B3Hash, RunId, Seq, TimeMs } from "../../wire";
import type { EventRecord, LogLine } from "../../wire";
import { lookOf } from "./entry";
import { lookOf as narrowingOf } from "./narrowing";

const RUN = RunId.make("00000000-0000-0000-0000-000000000001");

const RECORD: EventRecord = {
  run: RUN,
  seq: Seq.make(7),
  kind: "run_started",
  t: TimeMs.make(Date.UTC(2026, 9, 2, 0, 0, 1, 250)),
  who: "hall/mayor",
  prev: B3Hash.make("0".repeat(64)),
  v: 1,
  data: {},
};

const LINE: LogLine = { seq: Seq.make(7), t: null, level: "decide", module: "runtime", line: "no offload", run: null };

describe("a row of the record's timeline", () => {
  test("a ledger row opens to the record as written and hands its position to the press", () => {
    const toggled: number[] = [];
    const entry = { kind: "record", key: "r7", seq: RECORD.seq, t: RECORD.t, record: RECORD } as const;
    const closed = lookOf({ entry, day: null, open: false, onToggle: (seq) => toggled.push(seq) }, "en");
    const opened = lookOf({ entry, day: "2026-10-02", open: true, onToggle: () => undefined }, "en");
    expect(closed.row).toMatchObject({ kind: "record", open: false, wire: { type: "button", "aria-expanded": false }, raw: undefined });
    expect(opened.row).toMatchObject({ kind: "record", open: true, wire: { type: "button", "aria-expanded": true }, raw: JSON.stringify(RECORD, null, 2) });
    expect([closed.day, opened.day]).toEqual([undefined, { day: "2026-10-02", zone: "UTC · ledger position" }]);
    const row = closed.row;
    if (row.kind === "record") row.wire.onclick();
    expect(toggled).toEqual([7]);
  });

  test("a record with no room says who wrote it, and a log line held without a time writes none", () => {
    const record = lookOf({ entry: { kind: "record", key: "r7", seq: RECORD.seq, t: RECORD.t, record: RECORD }, day: null, open: false, onToggle: () => undefined }, "en");
    const line = lookOf({ entry: { kind: "log", key: "l7", seq: LINE.seq, t: null, line: LINE }, day: null, open: false, onToggle: () => undefined }, "en");
    expect(record.row).toMatchObject({ kind: "record", when: { text: "00:00:01.250Z", at: "2026-10-02T00:00:01.250Z" }, seq: "#7", where: "by hall/mayor" });
    expect(line.row).toEqual({ kind: "log", when: undefined, seq: "#7", level: "decide", line: "no offload", module: "runtime" });
  });
});

describe("a narrowing choice", () => {
  test("offers every one first, holds the empty value for it, and hands a pick on as null", () => {
    const picked: (string | null)[] = [];
    const look = narrowingOf({ label: "runs", options: [{ value: "a", label: "a" }], current: null, onPick: (value) => picked.push(value) }, "en");
    expect([look.options, look.wire.value]).toEqual([
      [
        { value: "", label: "all" },
        { value: "a", label: "a" },
      ],
      "",
    ]);
    look.wire.onchange({ currentTarget: { value: "a" } });
    look.wire.onchange({ currentTarget: { value: "" } });
    expect(picked).toEqual(["a", null]);
  });
});
