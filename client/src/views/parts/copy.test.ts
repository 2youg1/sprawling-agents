// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The copy key's wiring, driven the way its seat drives it and with no
// look at all (client/Spec.lean §4-65). Eight hand-written copies of
// this behaviour used to disagree: one never cleared its previous
// receipt, three dropped a failed write without a word. These tests
// hold the one that replaced them.

import { describe, expect, test } from "bun:test";

import { say } from "../../core/lang";
import { lookOf, presser, RECEIPT_MS, REST } from "./copy";
import type { Board, Copied, Hands } from "./copy";

// A clipboard whose every write waits until the test settles it.
function board() {
  const writes: { text: string; land: () => void; refuse: (e: Error) => void }[] = [];
  const clip: Board = {
    writeText: (text) =>
      new Promise<void>((land, refuse) => {
        writes.push({ text, land, refuse });
      }),
  };
  return { clip, writes };
}

// A clock the test advances by hand.
function clock() {
  const waits: { run: () => void; ms: number; live: boolean }[] = [];
  const later: Hands["later"] = (run, ms) => {
    const wait = { run, ms, live: true };
    waits.push(wait);
    return () => {
      wait.live = false;
    };
  };
  const fire = (): void => {
    for (const wait of waits.splice(0)) if (wait.live) wait.run();
  };
  return { later, waits, fire };
}

const settle = (): Promise<void> => new Promise((done) => setTimeout(done, 0));

function rig(clip: Board | undefined) {
  const shown: Copied[] = [];
  const time = clock();
  const press = presser({ board: () => clip, later: time.later, show: (now) => shown.push(now) });
  return { shown, time, press };
}

describe("a copy key's receipt", () => {
  test("appears only after the write lands, and returns to rest after RECEIPT_MS", async () => {
    const { clip, writes } = board();
    const { shown, time, press } = rig(clip);
    press("cargo xtask gates");
    await settle();
    expect(shown).toEqual([]);
    writes[0]?.land();
    await settle();
    expect(shown).toEqual([{ kind: "copied" }]);
    expect(time.waits.map((wait) => wait.ms)).toEqual([RECEIPT_MS]);
    time.fire();
    expect(shown).toEqual([{ kind: "copied" }, REST]);
  });

  test("a second receipt restarts the wait instead of being cut short by the first", async () => {
    const { clip, writes } = board();
    const { shown, time, press } = rig(clip);
    press("one");
    writes[0]?.land();
    await settle();
    press("two");
    writes[1]?.land();
    await settle();
    // The first wait was cancelled: firing every wait returns to rest once.
    time.fire();
    expect(shown).toEqual([{ kind: "copied" }, { kind: "copied" }, REST]);
  });

  test("a refused write says the browser's reason and holds until the next press", async () => {
    const { clip, writes } = board();
    const { shown, time, press } = rig(clip);
    press("secret-free text");
    writes[0]?.refuse(new Error("Document is not focused."));
    await settle();
    expect(shown).toEqual([{ kind: "refused", why: { kind: "refused", said: "Document is not focused." } }]);
    expect(time.waits).toEqual([]);
  });

  test("a page without a clipboard is refused as such, not silently", async () => {
    const { shown, press } = rig(undefined);
    press("anything");
    await settle();
    expect(shown).toEqual([{ kind: "refused", why: { kind: "absent" } }]);
  });

  test("a slow first write that lands after a second press is dropped", async () => {
    const { clip, writes } = board();
    const { shown, press } = rig(clip);
    press("first");
    press("second");
    writes[1]?.land();
    await settle();
    writes[0]?.refuse(new Error("late"));
    await settle();
    expect(shown).toEqual([{ kind: "copied" }]);
  });
});

describe("what a look of the copy key is given", () => {
  const press = (): void => undefined;

  test("a named key quotes its text and carries no aria-label, since its name is visible", () => {
    const look = lookOf({ text: "just check" }, REST, "en", press);
    expect(look.key["aria-label"]).toBeUndefined();
    expect(look.note).toContain("just check");
    expect(look.said).toBe("");
  });

  test("a bare key names itself for a screen reader, and a refusal replaces the note with the reason", () => {
    const refused: Copied = { kind: "refused", why: { kind: "absent" } };
    const look = lookOf({ text: () => "a whole file", note: "the file", form: "bare" }, refused, "en", press);
    expect(look.key["aria-label"]).toBe(say("en", "copy_refused"));
    expect(look.note).toBe(say("en", "copy_no_clipboard"));
    expect(look.said).toBe(say("en", "copy_refused"));
    expect(look.refused).toBe(true);
  });
});
