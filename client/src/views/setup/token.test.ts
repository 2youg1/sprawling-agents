// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The bags any look of a colour token row spreads: the well wears an
// override on the token it names, and the way back exists only while an
// override holds and takes off that token's alone.

import { describe, expect, test } from "bun:test";

import { tokenOf } from "./token";
import type { TokenHands } from "./token";

function record(): { readonly hands: TokenHands; readonly done: string[] } {
  const done: string[] = [];
  return {
    done,
    hands: {
      pick: (name, value) => done.push(`pick ${name} ${value}`),
      unpick: (name) => done.push(`unpick ${name}`),
    },
  };
}

const RESET = { word: "default", label: "--color-accent back to default" };

describe("tokenOf", () => {
  test("a held override offers the way back, named for its token", () => {
    const { hands, done } = record();
    const look = tokenOf({ name: "--color-accent", drawn: "#3366ff", reset: RESET }, hands);
    look.well.onchange({ currentTarget: { value: "#ff0000" } });
    look.reset?.wire.onclick();
    expect([look.well.value, look.reset?.word, look.reset?.wire["aria-label"]]).toEqual([
      "#3366ff",
      "default",
      "--color-accent back to default",
    ]);
    expect(done).toEqual(["pick --color-accent #ff0000", "unpick --color-accent"]);
  });

  test("with no override there is no way back, and an unresolved colour stays empty", () => {
    const { hands } = record();
    const look = tokenOf({ name: "--color-edge", drawn: undefined, reset: undefined }, hands);
    expect([look.reset, look.well.value, look.well["aria-label"]]).toEqual([undefined, "", "--color-edge"]);
  });
});
