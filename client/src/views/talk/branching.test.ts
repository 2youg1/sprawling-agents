// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The one door every branch goes through: main moves when the belief
// holds the room's new session and not when the frame leaves the page,
// a refusal stays with the branch that met it, and a second press while
// one is opening sends nothing. The trace vectors are the ones `#eval`
// prints in client/spec/Views/Talk/Branching.lean, replayed through
// `keepBranches`; a difference between the model and the code turns
// the last test red.

import { describe, expect, test } from "bun:test";
import { get, writable } from "svelte/store";

import { createBelief } from "../../core/belief";
import type { Belief } from "../../core/belief/shape";
import type { View } from "../../core/route";
import { Address, RunId, Seq } from "../../wire";
import type { AxError, Command, Origin } from "../../wire";
import { branchClaims, keepBranches } from "./branching";
import type { Branching, Opening } from "./branching";

const ROOM = Address.make("lab/a");
const ORIGIN: Origin = {
  run: RunId.make("00000000-0000-4000-8000-000000000001"),
  at_seq: Seq.make(7),
};
const BRANCH: Opening = { kind: "branch", origin: ORIGIN };
const EMPTY: Belief = get(createBelief(() => 0).belief);

// The two facts of the model's `Seen`: the room's current session and
// the arrival number of the newest refusal, 0 for none. One refusal
// object per arrival number, because the page tells refusals apart by
// identity.
const refusals = new Map<number, AxError>();
function refusalNumbered(n: number): AxError {
  const held = refusals.get(n);
  if (held !== undefined) return held;
  const made: AxError = {
    code: "E_BUSY",
    action: "start a new session",
    subject: `${ROOM}: a run is running there`,
    recovery: "",
    nearby: [],
    retry: "no",
  };
  refusals.set(n, made);
  return made;
}

function seen(session: number, refusal: number): Belief {
  return {
    ...EMPTY,
    sessions: session === 0 ? {} : { [ROOM]: Seq.make(session) },
    refusal: refusal === 0 ? null : refusalNumbered(refusal),
  };
}

type Act = "send" | "go";

// A page with one belief, a socket that takes or drops the frame as the
// test says, and an address bar that remembers where it was sent.
function page(start: Belief) {
  const belief = writable(start);
  const acts: Act[] = [];
  const views: View[] = [];
  const sent: Command[] = [];
  let leaves = true;
  const branches = keepBranches({
    belief,
    send: (command) => {
      if (!leaves) return false;
      sent.push(command);
      acts.push("send");
      return true;
    },
    go: (view) => {
      views.push(view);
      acts.push("go");
    },
  });
  let went = 0;
  return {
    belief,
    acts,
    views,
    sent,
    branches,
    went: () => went,
    press(left: boolean): void {
      leaves = left;
      branches.open(ROOM, BRANCH, () => {
        went += 1;
      });
    },
  };
}

describe("branching, then going", () => {
  test("main stays until the belief holds the room's new session", () => {
    const at = page(seen(3, 0));
    at.press(true);
    // The idem is minted afresh per frame, so the frame is read without it.
    expect(
      at.sent.map((command) =>
        "open_session" in command ? { ...command.open_session, idem: null } : command,
      ),
    ).toEqual([{ addr: ROOM, carry: "nothing", from: ORIGIN, idem: null }]);
    expect(at.views).toEqual([]);
    expect(at.went()).toBe(0);
    expect(get(at.branches.held)).toEqual({
      kind: "opening",
      room: ROOM,
      opening: BRANCH,
      session: Seq.make(3),
      refusal: null,
    });
    at.belief.set(seen(9, 0));
    expect(at.views).toEqual([{ kind: "talk", address: ROOM }]);
    expect(at.went()).toBe(1);
    expect(get(at.branches.held)).toEqual({ kind: "idle" });
  });

  test("a refusal leaves main where it was, and the branch draws it rather than the corner", () => {
    const at = page(seen(3, 0));
    at.press(true);
    at.belief.set(seen(3, 1));
    expect(at.views).toEqual([]);
    expect(at.went()).toBe(0);
    const held: Branching = get(at.branches.held);
    expect(held).toEqual({
      kind: "refused",
      room: ROOM,
      opening: BRANCH,
      error: refusalNumbered(1),
    });
    expect(branchClaims(held, refusalNumbered(1))).toBe(true);
    expect(branchClaims(held, refusalNumbered(2))).toBe(false);
  });

  test("a refusal that arrived before the press is not this branch's answer", () => {
    const at = page(seen(3, 1));
    at.press(true);
    at.belief.set(seen(3, 1));
    expect(get(at.branches.held).kind).toBe("opening");
  });

  test("a second press while one branch is opening sends nothing", () => {
    const at = page(seen(3, 0));
    at.press(true);
    at.press(true);
    expect(at.acts).toEqual(["send"]);
  });

  test("a frame that never left the page opens nothing", () => {
    const at = page(seen(3, 0));
    at.press(false);
    expect(get(at.branches.held)).toEqual({ kind: "idle" });
    at.belief.set(seen(9, 0));
    expect(at.acts).toEqual([]);
  });

  // Each vector: the events, the phase the model ends in, and what it
  // did on the way, as `#eval vectors.map …` printed them.
  type Event =
    | { readonly press: readonly [number, number, boolean] }
    | { readonly arrive: readonly [number, number] };
  const VECTORS: readonly {
    readonly events: readonly Event[];
    readonly last: Branching["kind"];
    readonly acts: readonly Act[];
  }[] = [
    {
      events: [{ press: [3, 0, true] }, { arrive: [3, 0] }, { arrive: [9, 0] }],
      last: "idle",
      acts: ["send", "go"],
    },
    {
      events: [{ press: [3, 0, true] }, { arrive: [3, 1] }, { arrive: [9, 1] }],
      last: "refused",
      acts: ["send"],
    },
    {
      events: [
        { press: [3, 1, true] },
        { arrive: [3, 1] },
        { arrive: [3, 0] },
        { arrive: [3, 2] },
        { press: [3, 2, true] },
        { arrive: [9, 2] },
      ],
      last: "idle",
      acts: ["send", "send", "go"],
    },
    {
      events: [{ press: [3, 0, true] }, { press: [3, 0, true] }, { arrive: [9, 1] }],
      last: "idle",
      acts: ["send", "go"],
    },
    {
      events: [
        { press: [0, 0, false] },
        { arrive: [4, 0] },
        { press: [4, 0, true] },
        { arrive: [4, 0] },
      ],
      last: "opening",
      acts: ["send"],
    },
    {
      events: [
        { arrive: [2, 1] },
        { press: [2, 1, true] },
        { press: [2, 1, false] },
        { arrive: [5, 3] },
      ],
      last: "idle",
      acts: ["send", "go"],
    },
  ];

  test("follows the model on every trace vector it prints", () => {
    for (const vector of VECTORS) {
      const at = page(seen(0, 0));
      for (const event of vector.events) {
        if ("press" in event) {
          const [session, refusal, left] = event.press;
          at.belief.set(seen(session, refusal));
          at.press(left);
        } else {
          at.belief.set(seen(...event.arrive));
        }
      }
      expect({ last: get(at.branches.held).kind, acts: at.acts }).toEqual({
        last: vector.last,
        acts: [...vector.acts],
      });
    }
  });
});
