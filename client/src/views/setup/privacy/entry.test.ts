// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { IDLE, step, type Action, type Event, type Send, type Stage } from "./entry";

// The trace vectors `#eval` prints in client/spec/Views/Privacy.lean,
// transcribed: the start, the events, and the final stage and sends
// `run vectorOffered` gives. A difference between the model and `step`
// turns one of these red.
const offered = (action: Action): boolean => action !== "reconcile";

interface Vector {
  readonly start: Stage<number>;
  readonly events: readonly Event<number>[];
  readonly last: Stage<number>;
  readonly sends: readonly Send<number>[];
}

const press = (action: Action): Event<number> => ({ kind: "press", action });
const confirm = (idem: number): Event<number> => ({ kind: "confirm", idem });
const lost = (idem: number): Event<number> => ({ kind: "lost", idem });
const outcome = (idem: number, result: "running" | "done" | "refused"): Event<number> => ({ kind: "outcome", idem, result });
const cancel: Event<number> = { kind: "cancel" };

const VECTORS: readonly Vector[] = [
  { start: IDLE, events: [press("apply"), cancel], last: IDLE, sends: [] },
  {
    start: IDLE,
    events: [press("apply"), confirm(1), outcome(2, "done"), outcome(1, "running"), outcome(1, "done")],
    last: { kind: "settled", idem: 1 },
    sends: [{ action: "apply", idem: 1 }],
  },
  {
    start: IDLE,
    events: [press("restore"), confirm(3), lost(3), press("restore"), confirm(4), outcome(4, "refused")],
    last: { kind: "refused", idem: 4 },
    sends: [
      { action: "restore", idem: 3 },
      { action: "restore", idem: 4 },
    ],
  },
  { start: IDLE, events: [press("reconcile"), confirm(5)], last: IDLE, sends: [] },
  {
    start: { kind: "settled", idem: 1 },
    events: [press("apply"), press("restore"), confirm(6), press("apply"), confirm(7), cancel],
    last: { kind: "sending", action: "apply", idem: 6 },
    sends: [{ action: "apply", idem: 6 }],
  },
  { start: IDLE, events: [confirm(8), outcome(8, "done"), lost(8)], last: IDLE, sends: [] },
];

function run(start: Stage<number>, events: readonly Event<number>[]): { last: Stage<number>; sends: Send<number>[] } {
  let stage = start;
  const sends: Send<number>[] = [];
  for (const event of events) {
    const next = step(offered, stage, event);
    stage = next.stage;
    if (next.send !== null) sends.push(next.send);
  }
  return { last: stage, sends };
}

describe("a privacy entry", () => {
  test("follows the model on every trace vector it prints", () => {
    for (const vector of VECTORS) {
      expect(run(vector.start, vector.events)).toEqual({ last: vector.last, sends: [...vector.sends] });
    }
  });
});
