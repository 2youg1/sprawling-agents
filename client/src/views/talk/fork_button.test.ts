// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { Seq, TimeMs } from "../../wire";
import type { Turn } from "../../wire";
import { forkLookOf } from "./fork_button";
import type { ForkEntry } from "./forking";

const TURN: Turn = { calls: [], notes: [], number: 2, opened: Seq.make(7), t: TimeMs.make(1), timing: "measured" };
const ENTRY: ForkEntry = { kind: "turn", turn: TURN };
const WORDS = { label: "branch from here", hint: "the original stays as it is" };

// Every report the bag makes, in order: what the hand rests on, and
// what was picked.
function recorded(): { readonly heard: (ForkEntry | null)[]; readonly picked: ForkEntry[]; readonly look: ReturnType<typeof forkLookOf> } {
  const heard: (ForkEntry | null)[] = [];
  const picked: ForkEntry[] = [];
  const look = forkLookOf(ENTRY, WORDS, {
    pick: (entry) => picked.push(entry),
    hover: (entry) => heard.push(entry),
  });
  return { heard, picked, look };
}

describe("the branch action's wiring", () => {
  test("the hand resting on it names its entry, and leaving clears it, by pointer and by focus", () => {
    const { heard, look } = recorded();
    look.wire.onmouseenter();
    look.wire.onmouseleave();
    look.wire.onfocus();
    look.wire.onblur();
    expect(heard).toEqual([ENTRY, null, ENTRY, null]);
  });

  test("a press branches from the entry it stands on and says nothing about the hand", () => {
    const { heard, picked, look } = recorded();
    look.wire.onclick();
    expect({ heard, picked }).toEqual({ heard: [], picked: [ENTRY] });
  });

  test("the look gets the words as given and a button to spread them on", () => {
    const { look } = recorded();
    expect({ label: look.label, hint: look.hint, type: look.wire.type }).toEqual({ ...WORDS, type: "button" });
  });
});
