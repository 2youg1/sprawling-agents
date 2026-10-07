// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the control decides before anything is drawn: where a track
// begins and ends, where an arrow key lands, which cell a keyboard
// arrives at, and the wire bags a look spreads - the key table and the
// `aria-*` values of client/Spec.lean §7-4, read off the value the seat
// hands any look (client D95). No look is imported here, so a look
// swapped for another leaves every one of these cases standing. The
// drawing itself is measured in a real engine by `cargo xtask render`
// against the gallery fixtures, because where a box landed is not
// something a unit test can be told.

import { describe, expect, test } from "bun:test";
import type { Attachment } from "svelte/attachments";

import {
  bands,
  landing,
  lookOf,
  nextStop,
  tabStop,
  type CellLook,
  type Choice,
  type Group,
  type Hands,
  type SegmentedLook,
  type SegmentedProps,
} from "./segmented";

type Api = "chat" | "responses" | "messages";

const OPEN_AI: Group = { label: "OpenAI", tone: "plain" };
const ANTHROPIC: Group = { label: "Anthropic", tone: "alert" };

// The three dialects as the provider form offers them: two from one
// lab, one from another, and the middle one not yet carried.
const CHAT: Choice<Api> = { value: "chat", label: "chat", group: OPEN_AI };
const RESPONSES: Choice<Api> = {
  value: "responses",
  label: "responses",
  group: OPEN_AI,
  why: "next version",
};
const MESSAGES: Choice<Api> = { value: "messages", label: "messages", group: ANTHROPIC };
const DIALECTS: readonly Choice<Api>[] = [CHAT, RESPONSES, MESSAGES];

// The same three values with nothing said about where they come from.
const PLAIN: readonly Choice<Api>[] = [
  { value: "chat", label: "chat" },
  { value: "responses", label: "responses" },
  { value: "messages", label: "messages" },
];

const REFUSED: readonly Choice<Api>[] = PLAIN.map((choice) => ({ ...choice, why: "next version" }));

describe("a flat list is cut into the tracks that are drawn", () => {
  test("cells that state no group are one track", () => {
    expect(bands(PLAIN)).toEqual([{ group: undefined, from: 0, cells: PLAIN }]);
  });

  test("a change of group opens a track and records where it starts", () => {
    expect(bands(DIALECTS)).toEqual([
      { group: OPEN_AI, from: 0, cells: [CHAT, RESPONSES] },
      { group: ANTHROPIC, from: 2, cells: [MESSAGES] },
    ]);
  });

  test("one name under two tones draws two tracks rather than one of them", () => {
    const split: readonly Choice<Api>[] = [
      { value: "chat", label: "chat", group: { label: "OpenAI", tone: "plain" } },
      { value: "responses", label: "responses", group: { label: "OpenAI", tone: "alert" } },
    ];
    expect(bands(split).length).toBe(2);
  });

  test("the tracks together are the list the caller gave, in order", () => {
    expect(bands(DIALECTS).flatMap((band) => band.cells)).toEqual([...DIALECTS]);
  });

  test("an empty control draws no track", () => {
    expect(bands<Api>([])).toEqual([]);
  });
});

describe("an arrow key crosses the whole control", () => {
  test("the right arrow wraps past the last cell to the first", () => {
    expect(nextStop(PLAIN, 2, 1)).toBe(0);
  });

  test("the left arrow wraps past the first cell to the last", () => {
    expect(nextStop(PLAIN, 0, -1)).toBe(2);
  });

  test("a cell that cannot be chosen is stepped over, not landed on", () => {
    expect(nextStop(DIALECTS, 0, 1)).toBe(2);
    expect(nextStop(DIALECTS, 2, -1)).toBe(0);
  });

  test("a control whose every cell is refused stays where it is", () => {
    expect(nextStop(REFUSED, 1, 1)).toBe(1);
  });

  test("a control with one cell answers that cell", () => {
    expect(nextStop([CHAT], 0, 1)).toBe(0);
  });
});

describe("a keyboard reaches the control in one stop", () => {
  test("the stop is the chosen cell", () => {
    expect(tabStop(DIALECTS, "messages")).toBe(2);
  });

  test("a held value no cell carries falls to the first choosable cell", () => {
    expect(tabStop([RESPONSES, MESSAGES], "chat")).toBe(1);
  });

  test("a control where every cell is refused still offers its reason", () => {
    expect(tabStop<string>(REFUSED, "nothing")).toBe(0);
  });

  test("an empty control offers nothing", () => {
    expect(tabStop<Api>([], "chat")).toBe(-1);
  });
});

describe("a key lands where the APG Radio Group table says", () => {
  test("down steps like right and up steps like left, over a refused cell", () => {
    expect([
      landing(DIALECTS, 0, "ArrowRight"),
      landing(DIALECTS, 0, "ArrowDown"),
      landing(DIALECTS, 2, "ArrowLeft"),
      landing(DIALECTS, 2, "ArrowUp"),
    ]).toEqual([2, 2, 0, 0]);
  });

  test("space selects the focused cell, and enter is left to the button's own click", () => {
    expect([landing(PLAIN, 1, " "), landing(PLAIN, 1, "Enter"), landing(PLAIN, 1, "Home")]).toEqual([
      1,
      undefined,
      undefined,
    ]);
  });
});

// A seat with no elements: it records every pick and every focus it is
// asked for, and hands one attachment per value as the seat does.
interface Seat {
  readonly picked: Api[];
  readonly focused: Api[];
  readonly hands: Hands<Api>;
}

function seat(): Seat {
  const picked: Api[] = [];
  const focused: Api[] = [];
  const holds = new Map<Api, Attachment<HTMLElement>>();
  return {
    picked,
    focused,
    hands: {
      focus: (value) => {
        focused.push(value);
      },
      hold: (value) => {
        const kept = holds.get(value);
        if (kept !== undefined) return kept;
        const made: Attachment<HTMLElement> = () => undefined;
        holds.set(value, made);
        return made;
      },
    },
  };
}

function drawn(at: Seat, held: Api | null, options: readonly Choice<Api>[] = DIALECTS): SegmentedLook {
  const props: SegmentedProps<Api> = {
    label: "dialect",
    options,
    held,
    onPick: (value) => {
      at.picked.push(value);
    },
  };
  return lookOf(props, "s1", at.hands);
}

function cells(look: SegmentedLook): readonly CellLook[] {
  return look.bands.flatMap((band) => band.cells);
}

// A key press the handler can be given, with a record of whether the
// handler took the key from the platform.
function press(key: string): { readonly event: { key: string; preventDefault: () => void }; taken: () => boolean } {
  let prevented = false;
  return {
    event: {
      key,
      preventDefault: () => {
        prevented = true;
      },
    },
    taken: () => prevented,
  };
}

describe("the wire bags carry the key table to any look", () => {
  test("the track is one named radiogroup, and every cell a radio", () => {
    const look = drawn(seat(), "chat");
    expect(look.group).toEqual({ role: "radiogroup", "aria-label": "dialect" });
    expect(cells(look).map((cell) => cell.wire.role)).toEqual(["radio", "radio", "radio"]);
  });

  test("an arrow selects the next choosable cell, asks for focus there, and takes the key", () => {
    const at = seat();
    const key = press("ArrowRight");
    cells(drawn(at, "chat"))[0]?.wire.onkeydown(key.event);
    expect({ picked: at.picked, focused: at.focused, taken: key.taken() }).toEqual({
      picked: ["messages"],
      focused: ["messages"],
      taken: true,
    });
  });

  test("space on a refused cell takes the key and selects nothing", () => {
    const at = seat();
    const key = press(" ");
    cells(drawn(at, "chat"))[1]?.wire.onkeydown(key.event);
    expect({ picked: at.picked, focused: at.focused, taken: key.taken() }).toEqual({
      picked: [],
      focused: [],
      taken: true,
    });
  });

  test("a key the control does not answer is left to the page", () => {
    const at = seat();
    const key = press("Tab");
    cells(drawn(at, "chat"))[0]?.wire.onkeydown(key.event);
    expect({ picked: at.picked, taken: key.taken() }).toEqual({ picked: [], taken: false });
  });

  test("a click on a refused cell selects nothing, and on a free one selects it", () => {
    const at = seat();
    const look = drawn(at, "chat");
    cells(look)[1]?.wire.onclick();
    cells(look)[2]?.wire.onclick();
    expect(at.picked).toEqual(["messages"]);
  });

  test("exactly one cell is a tab stop, the one tabStop names", () => {
    const stops = (held: Api | null, options: readonly Choice<Api>[]): readonly number[] =>
      cells(drawn(seat(), held, options)).map((cell) => cell.wire.tabindex);
    expect([stops("messages", DIALECTS), stops(null, DIALECTS), stops("chat", REFUSED)]).toEqual([
      [-1, -1, 0],
      [0, -1, -1],
      [0, -1, -1],
    ]);
  });

  test("the held cell is checked, and a refused cell is disabled and described by its reason", () => {
    const look = drawn(seat(), "messages");
    expect(
      cells(look).map((cell) => ({
        state: cell.state,
        checked: cell.wire["aria-checked"],
        disabled: cell.wire["aria-disabled"],
        described: cell.wire["aria-describedby"],
        why: cell.why,
      })),
    ).toEqual([
      { state: "free", checked: false, disabled: false, described: undefined, why: undefined },
      { state: "refused", checked: false, disabled: true, described: "s1-why-1", why: { id: "s1-why-1", text: "next version" } },
      { state: "held", checked: true, disabled: false, described: undefined, why: undefined },
    ]);
  });

  test("each band says where its slider stands and which tone paints it", () => {
    expect(
      drawn(seat(), "messages").bands.map((band) => ({ key: band.key, heading: band.heading, tone: band.tone, held: band.held })),
    ).toEqual([
      { key: 0, heading: "OpenAI", tone: "plain", held: -1 },
      { key: 2, heading: "Anthropic", tone: "alert", held: 0 },
    ]);
  });

  test("every cell carries the same attachment on every draw, so a redraw keeps its element", () => {
    const at = seat();
    const symbols = (look: SegmentedLook): readonly unknown[] =>
      cells(look).flatMap((cell) => Object.getOwnPropertySymbols(cell.wire).map((key) => cell.wire[key]));
    const first = symbols(drawn(at, "chat"));
    expect(first.length).toBe(3);
    expect(symbols(drawn(at, "messages"))).toEqual(first);
  });
});
