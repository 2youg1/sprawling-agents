// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The chosen session's timeline as its look receives it
// (client/Spec.lean §7K): Ledger order inside a turn, a date said once
// for each later day, and what a call and a checkpoint row do.

import { describe, expect, test } from "bun:test";

import { Address, GitOid, RunId, Seq, TimeMs, UsdMicros } from "../../wire";
import type { Call, CommitAnswer, Turn } from "../../wire";
import { timelineOf } from "./timeline";
import type { TimelineHands } from "./timeline";

const DAY = 86_400_000;
const T0 = Date.UTC(2026, 9, 7, 23, 59, 0);
const OID = GitOid.make("a".repeat(40));
const PARENT = GitOid.make("b".repeat(40));

const call = (at: number, answered: number): Call => ({
  at: Seq.make(at),
  tool: "exec",
  subject: "cargo test",
  outcome: "answered",
  called: TimeMs.make(answered - 1_000),
  answered: TimeMs.make(answered),
  timing: "measured",
});

const turn: Turn = {
  calls: [call(14, T0 + DAY), call(11, T0 + 10_000)],
  notes: [{ checkpointed: { at: Seq.make(12), oid: OID } }],
  number: 1,
  opened: Seq.make(10),
  t: TimeMs.make(T0),
  timing: "measured",
};

const commit: CommitAnswer = {
  actor: Address.make("lab/room1"),
  at: TimeMs.make(T0 + DAY / 2),
  lineage: [],
  model: "m",
  oid: OID,
  parents: [PARENT],
  run: RunId.make("00000000-0000-4000-8000-000000000001"),
  seq: Seq.make(12),
  spent: UsdMicros.make(0),
};

function hands(log: string[], open: boolean): TimelineHands {
  return {
    open: open ? (at) => log.push(`open ${String(at)}`) : undefined,
    pick: (picked) => log.push(`pick ${picked.oid.slice(0, 3)}`),
    list: () => undefined,
  };
}

describe("the timeline's rows", () => {
  test("a turn, then its calls and checkpoints in the order the Ledger wrote them", () => {
    const look = timelineOf({ lang: "en", turns: [turn], commits: [commit], picked: null }, hands([], true));
    expect(look.rows.map((row) => row.key)).toEqual(["t10", "c11", `k${OID}`, "c14"]);
  });

  test("the head says the first day, and a row on a later day says its date once, above it", () => {
    const look = timelineOf({ lang: "en", turns: [turn], commits: [commit], picked: null }, hands([], true));
    expect({ head: look.day, days: look.rows.map((row) => row.day) }).toEqual({
      head: "2026-10-07 · UTC",
      days: [undefined, undefined, "2026-10-08 · UTC", undefined],
    });
  });

  test("a call opens on the right side only while the session holds a run; the picked checkpoint says so", () => {
    const log: string[] = [];
    const held = timelineOf({ lang: "en", turns: [turn], commits: [commit], picked: OID }, hands(log, true));
    const none = timelineOf({ lang: "en", turns: [turn], commits: [], picked: null }, hands(log, false));
    const [, first, checkpoint] = held.rows;
    if (first?.kind === "call") first.wire.onclick();
    if (checkpoint?.kind === "checkpoint") checkpoint.wire.onclick();
    const [, unheld, orphan] = none.rows;
    expect({
      log,
      current: checkpoint?.kind === "checkpoint" ? checkpoint.wire["aria-current"] : null,
      produced: checkpoint?.kind === "checkpoint" ? checkpoint.produced : null,
      unheld: unheld?.kind === "call" ? unheld.wire.disabled : null,
      orphan: orphan?.kind === "checkpoint" ? [orphan.wire.disabled, orphan.at] : null,
    }).toEqual({
      log: ["open 11", "pick aaa"],
      current: "true",
      produced: { base: PARENT, head: OID },
      unheld: true,
      orphan: [true, "—"],
    });
  });
});
