// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { get, readable } from "svelte/store";

import { Address, Seq, Tag } from "../wire";
import type { Answer, Command, SessionTags } from "../wire";
import type { Asking } from "./asking";
import { PIN, given, inUse, keepTags, readTag, retagged, stripped } from "./tags";

const CITY = Address.make("harbour");
const MAYOR_ROOM = Address.make("hall/mayor");
const at = (began: number): { city: Address; room: Address; began: Seq } => ({ city: CITY, room: MAYOR_ROOM, began: Seq.make(began) });
const tags = (...words: string[]): Tag[] => words.map((word) => Tag.make(word));

describe("the tags a person gives sessions", () => {
  test("what a person types is folded to lower case, and a word that is no tag is nothing", () => {
    expect(readTag("  Bug ")).toBe(Tag.make("bug"));
    expect(readTag("重构")).toBe(Tag.make("重构"));
    expect([readTag(""), readTag("two words"), readTag("#x"), readTag("x".repeat(25))]).toEqual([null, null, null, null]);
  });

  test("giving and stripping answer the session's whole new set", () => {
    const held: SessionTags[] = [{ ...at(3), tags: tags("bug") }];
    expect(given(held, at(3), PIN)).toEqual({ ...at(3), tags: tags("bug", "pin") });
    expect(given(held, at(3), Tag.make("bug"))).toEqual({ ...at(3), tags: tags("bug") });
    expect(stripped(held, at(3), Tag.make("bug"))).toEqual({ ...at(3), tags: [] });
    expect(given(held, at(9), Tag.make("x"))).toEqual({ ...at(9), tags: tags("x") });
  });

  test("a set replaces the session's set, and an empty one removes it", () => {
    const held: SessionTags[] = [{ ...at(3), tags: tags("bug") }, { ...at(9), tags: tags("x") }];
    expect(retagged(held, { ...at(3), tags: tags("y") })).toEqual([{ ...at(9), tags: tags("x") }, { ...at(3), tags: tags("y") }]);
    expect(retagged(held, { ...at(3), tags: [] })).toEqual([{ ...at(9), tags: tags("x") }]);
  });

  test("the filter row offers this city's tags once each, without the pin", () => {
    const held: SessionTags[] = [
      { ...at(3), tags: tags("pin", "zeta") },
      { ...at(9), tags: tags("alpha", "zeta") },
      { city: Address.make("fjord"), room: MAYOR_ROOM, began: Seq.make(3), tags: tags("elsewhere") },
    ];
    expect(inUse(held, CITY)).toEqual(tags("alpha", "zeta"));
  });

  test("the city's answer is what the page holds, and a change is drawn once it is sent", () => {
    const sent: Command[] = [];
    let live = true;
    const answer: Answer = { preferences: { tags: [{ ...at(3), tags: tags("bug") }] } };
    const asking: Asking = {
      ask: () => readable<Answer | undefined>(answer),
      refresh: () => undefined,
      answered: () => undefined,
      invalidate: () => undefined,
      reconnected: () => undefined,
      resumed: () => undefined,
    };
    const kept = keepTags({
      asking,
      command: (command) => {
        if (live) sent.push(command);
        return live;
      },
    });
    expect(get(kept.held)).toEqual([{ ...at(3), tags: tags("bug") }]);
    expect(kept.retag({ ...at(3), tags: tags("bug", "pin") })).toBe(true);
    expect(get(kept.held)).toEqual([{ ...at(3), tags: tags("bug", "pin") }]);
    expect(sent.map((frame) => ("put_preferences" in frame ? frame.put_preferences.patch : null))).toEqual([
      { tags: { ...at(3), tags: tags("bug", "pin") } },
    ]);
    live = false;
    expect(kept.retag({ ...at(3), tags: [] })).toBe(false);
    expect(get(kept.held)).toEqual([{ ...at(3), tags: tags("bug", "pin") }]);
  });
});
