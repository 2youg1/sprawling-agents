// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The monitor's wiring, read without its look: what each toggle says to
// a screen reader and what pressing it asks of the seat, which keys and
// gestures in a pane pause following, and which `F` takes it up again.

import { describe, expect, test } from "bun:test";

import { lookOf, resumes } from "./monitor";
import type { Following, Hands, MonitorState, Pane } from "./monitor";

interface Asked {
  readonly follows: Following[];
  readonly folds: Pane[];
  readonly jumps: string[];
  readonly hands: Hands;
}

function asked(): Asked {
  const follows: Following[] = [];
  const folds: Pane[] = [];
  const jumps: string[] = [];
  const hold = (): void => undefined;
  return {
    follows,
    folds,
    jumps,
    hands: {
      follow: (next) => follows.push(next),
      fold: (next) => folds.push(next),
      jump: (path) => jumps.push(path),
      holdCode: hold,
      holdRecord: hold,
    },
  };
}

const OPEN: MonitorState = { following: "following", terminal: "open", paths: ["src/a.rs", "src/b.rs"] };
const FOLDED: MonitorState = { following: "paused", terminal: "folded", paths: ["src/a.rs", "src/b.rs"] };

describe("the toggles", () => {
  test("following is a pressed toggle, and pressing it pauses", () => {
    const { follows, hands } = asked();
    const look = lookOf(OPEN, "en", hands);
    expect(look.follow.wire["aria-pressed"]).toBe(true);
    look.follow.wire.onclick();
    expect(follows).toEqual(["paused"]);
  });

  test("paused is an unpressed toggle, and pressing it follows again", () => {
    const { follows, hands } = asked();
    const look = lookOf(FOLDED, "en", hands);
    expect(look.follow.wire["aria-pressed"]).toBe(false);
    look.follow.wire.onclick();
    expect(follows).toEqual(["following"]);
  });

  test("the fold toggle says whether the terminal is open and asks for the other state", () => {
    const open = asked();
    const opened = lookOf(OPEN, "en", open.hands);
    expect(opened.fold.wire["aria-expanded"]).toBe(true);
    opened.fold.wire.onclick();
    const folded = asked();
    const closed = lookOf(FOLDED, "en", folded.hands);
    expect(closed.fold.wire["aria-expanded"]).toBe(false);
    closed.fold.wire.onclick();
    expect([...open.folds, ...folded.folds]).toEqual(["folded", "open"]);
  });
});

describe("the panes", () => {
  test("open draws the terminal pane and no file column; folded the other way round", () => {
    const { hands } = asked();
    const opened = lookOf(OPEN, "en", hands);
    expect([opened.record?.["aria-label"], opened.index]).toEqual(["terminal", undefined]);
    const closed = lookOf(FOLDED, "en", hands);
    expect([closed.record, closed.index?.files.map((file) => file.path)]).toEqual([undefined, ["src/a.rs", "src/b.rs"]]);
  });

  test("a file in the folded column asks the seat to jump to it", () => {
    const { jumps, hands } = asked();
    lookOf(FOLDED, "en", hands).index?.files[1]?.wire.onclick();
    expect(jumps).toEqual(["src/b.rs"]);
  });

  test("a wheel turn, a touch drag or a scrolling key in a pane pauses; another key does not", () => {
    const { follows, hands } = asked();
    const { code } = lookOf(OPEN, "en", hands);
    code.onwheel();
    code.ontouchmove();
    code.onkeydown({ key: "PageDown" });
    code.onkeydown({ key: " " });
    code.onkeydown({ key: "a" });
    expect(follows).toEqual(["paused", "paused", "paused", "paused"]);
    expect([code.role, code.tabindex]).toEqual(["region", 0]);
  });
});

describe("taking following up again", () => {
  const bare = { ctrlKey: false, metaKey: false, altKey: false, typing: false };

  test("F in either case resumes", () => {
    expect([resumes({ ...bare, key: "f" }), resumes({ ...bare, key: "F" })]).toEqual([true, true]);
  });

  test("a modified F, an F typed into a field, or another key does not", () => {
    expect([
      resumes({ ...bare, key: "f", ctrlKey: true }),
      resumes({ ...bare, key: "f", metaKey: true }),
      resumes({ ...bare, key: "f", altKey: true }),
      resumes({ ...bare, key: "f", typing: true }),
      resumes({ ...bare, key: "g" }),
    ]).toEqual([false, false, false, false, false]);
  });
});
