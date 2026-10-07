// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The head of a step of the guide as a disclosure button (client/Spec.lean
// §7G): it controls the body it opens, says whether that body is open,
// and a press reaches the guide.

import { describe, expect, test } from "bun:test";

import { say } from "../../core/lang";
import { bodyId, stepLookOf } from "./step";

describe("a step's head", () => {
  test("it controls its body, says whether it is open, and says where the step stands", () => {
    let pressed = 0;
    const look = stepLookOf(
      { step: "dependencies", at: 1, standing: "skipped", open: true, id: "w-dependencies", lang: "en" },
      () => {
        pressed += 1;
      },
    );
    look.wire.onclick();
    expect({ ...look, wire: { ...look.wire, onclick: undefined } }).toEqual({
      number: "02",
      title: say("en", "guide_step_dependencies"),
      standing: "skipped",
      word: say("en", "guide_standing_skipped"),
      wire: {
        id: "w-dependencies",
        type: "button",
        "aria-expanded": true,
        "aria-controls": bodyId("w-dependencies"),
        onclick: undefined,
      },
    });
    expect(pressed).toBe(1);
  });

  test("a step nobody touched says nothing about where it stands", () => {
    const look = stepLookOf(
      { step: "mcp", at: 4, standing: "untouched", open: false, id: "w-mcp", lang: "en" },
      () => undefined,
    );
    expect([look.word, look.wire["aria-expanded"]]).toEqual([undefined, false]);
  });
});
