// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The palette's box as an APG Combobox: the keys a person presses in
// it, what each one asks the seat to do, and the ARIA it carries, read
// from the value every look is handed rather than from any look.

import { describe, expect, test } from "bun:test";

import { completed } from "../core/completion";
import type { BoxHands, BoxState } from "./palette";
import { boxOf, carryPlaces, sessionPlaces, whyFor } from "./palette";
import { Address, Seq, TimeMs } from "../wire";
import type { SessionLine } from "../wire";
import type { Entry } from "./palette/entry";
import { rowId, rowsLookOf } from "./palette/rows";

interface Asked {
  readonly wrote: string[];
  readonly pointed: number[];
  picked: number;
}

function hands(): { readonly asked: Asked; readonly hands: BoxHands } {
  const asked: Asked = { wrote: [], pointed: [], picked: 0 };
  return {
    asked,
    hands: {
      write: (line) => asked.wrote.push(line),
      point: (at) => asked.pointed.push(at),
      pick: () => {
        asked.picked += 1;
      },
    },
  };
}

// Whether an input method is composing when the key is pressed.
type Typing = "settled" | "composing";

function press(
  key: string,
  typing: Typing = "settled",
): {
  readonly key: string;
  readonly isComposing: boolean;
  prevented: boolean;
  readonly preventDefault: () => void;
} {
  const event = {
    key,
    isComposing: typing === "composing",
    prevented: false,
    preventDefault: () => {
      event.prevented = true;
    },
  };
  return event;
}

const AT: BoxState = {
  asking: "places",
  query: "",
  cursor: 1,
  count: 3,
  list: "p-list",
  lang: "en",
};

describe("the palette's box", () => {
  test("↓ and ↑ move the cursor, stop at the ends, and keep the key from the caret", () => {
    const down = hands();
    const pressed = press("ArrowDown");
    boxOf(AT, down.hands).onkeydown(pressed);
    boxOf({ ...AT, cursor: 2 }, down.hands).onkeydown(press("ArrowDown"));
    boxOf({ ...AT, cursor: 0 }, down.hands).onkeydown(press("ArrowUp"));
    expect(down.asked.pointed).toEqual([2, 2, 0]);
    expect(pressed.prevented).toBe(true);
  });

  test("on an empty list the cursor stays on the first row to arrive", () => {
    const empty = hands();
    const box = boxOf({ ...AT, cursor: 0, count: 0 }, empty.hands);
    box.onkeydown(press("ArrowDown"));
    expect(empty.asked.pointed).toEqual([0]);
    expect(box["aria-expanded"]).toBe(false);
    expect(box["aria-activedescendant"]).toBeUndefined();
  });

  test("Enter takes the row under the cursor", () => {
    const enter = hands();
    const pressed = press("Enter");
    boxOf(AT, enter.hands).onkeydown(pressed);
    expect(enter.asked.picked).toBe(1);
    expect(pressed.prevented).toBe(true);
  });

  test("a key pressed while an input method composes is the input method's: Enter picks no row, Tab completes no verb", () => {
    const composing = hands();
    const keys = [
      press("Enter", "composing"),
      press("ArrowDown", "composing"),
      press("Tab", "composing"),
    ];
    for (const each of keys) boxOf({ ...AT, query: "/hal" }, composing.hands).onkeydown(each);
    expect(composing.asked).toEqual({ wrote: [], pointed: [], picked: 0 });
    expect(keys.map((each) => each.prevented)).toEqual([false, false, false]);
  });

  test("Tab completes a verb, and is the focus's own key on a place", () => {
    const verb = hands();
    const onVerb = press("Tab");
    boxOf({ ...AT, query: "/hal" }, verb.hands).onkeydown(onVerb);
    expect(verb.asked.wrote).toEqual([completed("/hal")]);
    expect(onVerb.prevented).toBe(true);

    const place = hands();
    const onPlace = press("Tab");
    boxOf({ ...AT, query: "cit" }, place.hands).onkeydown(onPlace);
    expect(place.asked.wrote).toEqual([]);
    expect(onPlace.prevented).toBe(false);
  });

  test("a letter is the box's own, and typing writes the line", () => {
    const typing = hands();
    const letter = press("a");
    const box = boxOf(AT, typing.hands);
    box.onkeydown(letter);
    box.oninput({ currentTarget: { value: "ci" } });
    expect(letter.prevented).toBe(false);
    expect(typing.asked).toEqual({ wrote: ["ci"], pointed: [], picked: 0 });
  });

  test("the box names the row the list draws under the cursor", () => {
    const entries: Entry[] = ["a", "b", "c"].map((label) => ({
      label,
      hint: "",
      act: () => undefined,
    }));
    const box = boxOf(AT, hands().hands);
    const rows = rowsLookOf(
      { listing: { kind: "places" }, shown: entries, cursor: AT.cursor, id: AT.list, lang: "en" },
      { hover: () => undefined, pick: () => undefined },
    );
    const active = rows.sections
      .flatMap((section) => section.rows)
      .filter((row) => row.wire["aria-selected"]);
    expect(active.map((row) => row.wire.id)).toEqual([box["aria-activedescendant"] ?? ""]);
    expect(box["aria-controls"]).toBe(rows.list.id);
    expect(box["aria-activedescendant"]).toBe(rowId(AT.list, 1));
  });
});

describe("why a verb cannot run from the palette", () => {
  const nowhere = { here: null, live: false, models: 0 };
  const room = { here: "hall/mayor", live: false, models: 2 };

  test("each capability a verb misses is named, and a verb that needs none is free", () => {
    expect(
      ["/new", "/diff", "/stop", "/steer", "/model", "/halt"].map((verb) => [
        verb,
        whyFor(verb, nowhere),
      ]),
    ).toEqual([
      ["/new", "palette_needs_room"],
      ["/diff", "palette_needs_room"],
      ["/stop", "no_run_in_front"],
      ["/steer", "no_run_in_front"],
      ["/model", "palette_needs_model"],
      ["/halt", undefined],
    ]);
  });

  test("a room or a run in front of the person is enough for /diff", () => {
    expect([whyFor("/diff", room), whyFor("/diff", { ...nowhere, live: true })]).toEqual([
      undefined,
      undefined,
    ]);
  });
});

describe("the sessions of the room main stands in", () => {
  const ROOM = Address.make("atlas/payments");
  const line = (began: number, name: string | null, preview: string | null): SessionLine => ({
    at: TimeMs.make(1_000),
    began: Seq.make(began),
    last: Seq.make(began + 5),
    runs: 1,
    name,
    preview,
    start: { opened: { carry: "nothing" } },
  });

  test("the current session goes to the room and an earlier one to its read-only view, each led by its number, the one shown saying so", () => {
    const places = sessionPlaces(
      { room: ROOM, viewing: Seq.make(23) },
      [line(40, "reconcile", null), line(23, null, "the ledger is short"), line(9, null, null)],
      "en",
      61_000,
    );
    expect(places).toEqual([
      { label: "#40 · reconcile", hint: "current session", to: { kind: "talk", address: ROOM } },
      {
        label: "#23 · the ledger is short",
        hint: "shown now · 1 min ago",
        to: { kind: "talk", address: ROOM, session: Seq.make(23) },
      },
      {
        label: "#9",
        hint: "session · 1 min ago",
        to: { kind: "talk", address: ROOM, session: Seq.make(9) },
      },
    ]);
  });
});

describe("carrying this conversation to another room", () => {
  const at = (room: string): Address => Address.make(room);

  test("each room the city knows, and a new one typed in the box, says before it is pressed why it cannot take the conversation", () => {
    const places = carryPlaces({
      here: at("lab/parser"),
      rooms: [at("lab/parser"), at("lab"), at("lab/review"), at("docs/notes")],
      typed: at("lab/lexer"),
      working: (room) => room === at("lab/review"),
    });
    expect(places).toEqual([
      { to: at("lab/parser"), why: "palette_carry_here" },
      { to: at("lab"), why: "palette_carry_building" },
      { to: at("lab/review"), why: "palette_carry_busy" },
      { to: at("docs/notes"), why: undefined },
      { to: at("lab/lexer"), why: undefined },
    ]);
  });

  test("a typed address the city already knows is not offered twice", () => {
    const places = carryPlaces({
      here: at("lab/parser"),
      rooms: [at("docs/notes")],
      typed: at("docs/notes"),
      working: () => false,
    });
    expect(places).toEqual([{ to: at("docs/notes"), why: undefined }]);
  });
});
