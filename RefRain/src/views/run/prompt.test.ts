// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { Address, B3Hash } from "../../wire";
import type { PrefixSegment } from "../../wire";
import { lookOf } from "./prompt";
import type { Hands } from "./prompt";

function segment(at: number, stored: boolean, dropped = 0): PrefixSegment {
  return {
    slot: "building",
    bytes: 2_048,
    hash: B3Hash.make(String(at).repeat(64)),
    sources: [{ addr: Address.make("shop/notes/Memo.md"), kept: 2_048, dropped }],
    stored,
    text: `segment ${String(at)}`,
  };
}

function recorder(): { readonly hands: Hands; readonly pressed: string[] } {
  const pressed: string[] = [];
  return {
    pressed,
    hands: {
      toggle: (hash) => pressed.push(`toggle ${hash.slice(0, 1)}`),
      visit: (source) => pressed.push(`visit ${source}`),
    },
  };
}

describe("the prompt lens", () => {
  test("is still asking until the city answers, and says so when it answers with nothing", () => {
    const { hands } = recorder();
    expect(lookOf(undefined, { open: new Set() }, "en", hands).segments).toBeUndefined();
    expect(lookOf({ segments: [], tools: null }, { open: new Set() }, "en", hands)).toEqual({
      tools: undefined,
      segments: [],
      none: "this run has no prompt on record",
    });
  });

  test("a segment's row opens and closes only itself, and its expanded state is the open set's", () => {
    const { hands, pressed } = recorder();
    const one = segment(1, true);
    const two = segment(2, true);
    const look = lookOf(
      { segments: [one, two], tools: null },
      { open: new Set([two.hash]) },
      "en",
      hands,
    );
    expect(
      look.segments?.map((each) => [each.open, each.fold["aria-expanded"], each.fold.type]),
    ).toEqual([
      [false, false, "button"],
      [true, true, "button"],
    ]);
    look.segments?.[0]?.fold.onclick();
    expect(pressed).toEqual(["toggle 1"]);
  });

  test("copy uses the exact text held for each segment", () => {
    const { hands } = recorder();
    const one = segment(1, true);
    const two = segment(2, true);
    const look = lookOf(
      {
        segments: [one, two],
        tools: null,
      },
      { open: new Set() },
      "en",
      hands,
    );
    expect(look.segments?.map((row) => row.copyText)).toEqual([one.text, two.text]);
  });

  test("a segment the store no longer holds says so and offers nothing to copy", () => {
    const { hands } = recorder();
    const [gone] =
      lookOf({ segments: [segment(3, false)], tools: null }, { open: new Set() }, "en", hands)
        .segments ?? [];
    expect([gone?.gone, gone?.copyText]).toEqual([
      "the store no longer holds these bytes",
      undefined,
    ]);
  });

  test("a source names what the budget cut only when it cut something, and opens its building", () => {
    const { hands, pressed } = recorder();
    const [kept, cut] = (
      lookOf(
        { segments: [segment(1, true), segment(2, true, 1_840)], tools: null },
        { open: new Set() },
        "en",
        hands,
      ).segments ?? []
    ).map((each) => each.sources[0]);
    expect([kept?.dropped, cut?.dropped]).toEqual([undefined, "1,840 bytes cut"]);
    cut?.onOpen();
    expect(pressed).toEqual(["visit shop/notes/Memo.md"]);
  });
});
