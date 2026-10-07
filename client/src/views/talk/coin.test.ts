// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { lookOf } from "./coin";
import type { Face } from "./coin_face";

// What a click on each face's wiring did: whether the stop was asked
// for, and whether the click was kept from submitting the form.
function clicked(face: Face): { readonly stopped: boolean; readonly held: boolean } {
  let stopped = false;
  let held = false;
  const look = lookOf(face, "dispatch", "en", () => {
    stopped = true;
  });
  const event = {
    preventDefault: () => {
      held = true;
    },
  };
  // @ts-expect-error a click handler is handed a whole MouseEvent; this one reads only preventDefault
  look.wire.onclick?.(event);
  return { stopped, held };
}

describe("the coin key's wiring", () => {
  // The send face lets the form's submit through, so Enter and a press
  // are one path; the stop face cancels and submits nothing; the faint
  // face swallows the press (client/spec/Views/Workspace.lean §7-11).
  test("each face's click does what its face says and nothing else", () => {
    expect((["send", "stop", "idle"] as const).map(clicked)).toEqual([
      { stopped: false, held: false },
      { stopped: true, held: false },
      { stopped: false, held: true },
    ]);
  });

  test("only the faint face is refused, and it says why", () => {
    const looks = (["send", "stop", "idle"] as const).map((face) => lookOf(face, "steer", "en", () => undefined));
    expect(looks.map((look) => [look.wire.type, look.wire["aria-disabled"], look.why !== undefined])).toEqual([
      ["submit", undefined, false],
      ["button", undefined, false],
      ["submit", "true", true],
    ]);
  });
});
