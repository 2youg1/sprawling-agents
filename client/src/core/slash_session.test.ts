// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { get, writable } from "svelte/store";
import type { Writable } from "svelte/store";

import { keepBranches } from "../views/talk/branching";
import { Address, RunId, Seq, Tag } from "../wire";
import type { AxError, Command, RunId as Run, SessionTags } from "../wire";
import { createBelief } from "./belief";
import type { Belief } from "./belief/shape";
import type { View } from "./route";
import { find, parse } from "./slash";
import type { SlashHands } from "./slash_hands";

// The verbs about the session in main: `/compact`, `/tag`, `/untag`, and
// where `/new` leaves main.
describe("the session in main", () => {
  const ROOM = Address.make("lab/room1");
  const GOING = RunId.make("00000000-0000-4000-8000-000000000007");
  const SESSION: SessionTags = { city: Address.make("harbour"), room: ROOM, began: Seq.make(5), tags: [Tag.make("bug")] };
  const EMPTY: Belief = get(createBelief(() => 0).belief);
  const BUSY: AxError = {
    code: "E_BUSY",
    action: "start a new session",
    subject: `${ROOM}: a run is running there`,
    recovery: "",
    nearby: [],
    retry: "no",
  };

  interface Held {
    readonly hands: SlashHands;
    // The page's belief, which the test moves as the city would.
    readonly belief: Writable<Belief>;
    readonly sent: Command[];
    readonly went: View[];
    readonly written: string[];
    readonly retagged: SessionTags[];
    // The run each `whenFrozen` waits on, and what it does then.
    readonly waiting: [Run, () => void][];
  }

  // Hands as a page fills them: `/new` and `/compact` open their session
  // through the page's one branch door, so main follows once the room
  // holds a newer session and a refusal keeps it where it was (#120).
  function held(live: boolean, tagged: SessionTags | null = SESSION): Held {
    const sent: Command[] = [];
    const went: View[] = [];
    const written: string[] = [];
    const retagged: SessionTags[] = [];
    const waiting: [Run, () => void][] = [];
    const belief = writable<Belief>({ ...EMPTY, sessions: { [ROOM]: Seq.make(3) } });
    const command = (frame: Command): boolean => {
      sent.push(frame);
      return true;
    };
    const go = (view: View): void => {
      went.push(view);
    };
    const branches = keepBranches({ belief, send: command, go });
    return {
      belief, sent, went, written, retagged, waiting,
      hands: {
        command,
        go,
        here: ROOM,
        live: live ? { run: GOING, at: Seq.make(9) } : null,
        newest: () => null,
        models: [],
        effort: null,
        setEffort: () => undefined,
        policy: { mode: "work", write: "create", admit: "tested", landing: "experiment" },
        setPolicy: () => undefined,
        goal: "",
        write: (line) => written.push(line),
        tagged,
        retag: (next) => {
          retagged.push(next);
          return true;
        },
        whenFrozen: (run, then) => waiting.push([run, then]),
        openSession: (room, carry) => branches.open(room, { kind: "fresh", carry }, () => undefined),
        opening: () => get(branches.held).kind === "opening",
      },
    };
  }

  function run(hands: SlashHands, line: string): void {
    const call = parse(line);
    const verb = call === null ? undefined : find(call.verb);
    expect(verb, line).toBeDefined();
    if (call !== null) verb?.run(hands, call);
  }

  const kinds = (sent: readonly Command[]): string[] => sent.flatMap((frame) => Object.keys(frame));

  test("/new from an earlier session brings main to the room once the city opened the new one", () => {
    const at = held(false);
    run(at.hands, "/new");
    const opened = at.sent.flatMap((frame) => ("open_session" in frame ? [[frame.open_session.addr, frame.open_session.carry, frame.open_session.from]] : []));
    expect([opened, at.went]).toEqual([[[ROOM, "nothing", null]], []]);
    at.belief.update((belief) => ({ ...belief, sessions: { [ROOM]: Seq.make(9) } }));
    expect(at.went).toEqual([{ kind: "talk", address: ROOM }]);
  });

  test("/new that the city refuses leaves main where it was", () => {
    const at = held(false);
    run(at.hands, "/new");
    at.belief.update((belief) => ({ ...belief, refusal: BUSY }));
    expect(at.went).toEqual([]);
  });

  test("/compact with nothing running opens a session carrying the handoff, and main follows once it is open", () => {
    const at = held(false);
    run(at.hands, "/compact");
    const opened = at.sent.flatMap((frame) => ("open_session" in frame ? [[frame.open_session.addr, frame.open_session.carry]] : []));
    expect([opened, at.went]).toEqual([[[ROOM, "handoff"]], []]);
    at.belief.update((belief) => ({ ...belief, sessions: { [ROOM]: Seq.make(9) } }));
    expect(at.went).toEqual([{ kind: "talk", address: ROOM }]);
  });

  test("/compact stops the run going first and opens the session only once it froze", () => {
    const at = held(true);
    run(at.hands, "/compact");
    expect(kinds(at.sent)).toEqual(["cancel"]);
    expect(at.waiting.map(([run]) => run)).toEqual([GOING]);
    at.waiting[0]?.[1]();
    expect(kinds(at.sent)).toEqual(["cancel", "open_session"]);
    const opened = at.sent[1];
    expect(opened !== undefined && "open_session" in opened ? opened.open_session.carry : null).toBe("handoff");
  });

  test("/compact while another session is opening stops nothing and keeps the line", () => {
    const at = held(true);
    at.hands.openSession(Address.make("lab/room2"), "nothing");
    run(at.hands, "/compact");
    expect([kinds(at.sent), at.waiting, at.written]).toEqual([["open_session"], [], []]);
  });

  test("/tag and /untag change the session in main by one word, folded to lower case", () => {
    const at = held(false);
    run(at.hands, "/tag Later");
    run(at.hands, "/untag bug");
    expect(at.retagged).toEqual([
      { ...SESSION, tags: [Tag.make("bug"), Tag.make("later")] },
      { ...SESSION, tags: [] },
    ]);
    expect(at.written).toEqual(["", ""]);
  });

  test("a word that is no tag, or no session in main, sends nothing and keeps the line", () => {
    const at = held(false);
    run(at.hands, "/tag two words");
    run(held(false, null).hands, "/tag fine");
    expect([at.retagged, at.written]).toEqual([[], []]);
  });
});
