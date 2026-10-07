// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The hint's wiring as client/Spec.lean §7-3 states it: a `role="tooltip"`
// node whose id the control is handed, Escape as its one key, and
// hold-to-reveal reaching only a hint that asked for it and was not
// dismissed. Where the box lands is measured by `cargo xtask render`
// against the two placement fixtures on `#/gallery`.

import { describe, expect, test } from "bun:test";

import { dismisses, lookOf, type TipState } from "./tip";

const AT_REST: TipState = { uid: "s7", dismissed: false, reengage: () => undefined };
const DISMISSED: TipState = { ...AT_REST, dismissed: true };

describe("the hint node", () => {
  test("is a tooltip named by the seat's own id when the caller names none", () => {
    const look = lookOf({ text: "halted" }, AT_REST);
    expect(look.hint).toEqual({ id: "s7", role: "tooltip", "data-exposable": undefined });
    expect(look.id).toBe("s7");
  });

  test("takes the id a caller's wiring already wrote into aria-describedby", () => {
    const look = lookOf({ text: "halted", id: "s3-why" }, AT_REST);
    expect(look.hint.id).toBe("s3-why");
    expect(look.id).toBe("s3-why");
    expect(look.anchor).toBe("--tip-s3-why");
  });

  test("stands above unless the caller asks for the right, and leaves the branch to the engine", () => {
    expect(lookOf({ text: "x" }, AT_REST)).toMatchObject({ side: "above", placing: "by-engine" });
    expect(lookOf({ text: "x", side: "right" }, AT_REST).side).toBe("right");
  });
});

describe("Escape and hold-to-reveal", () => {
  test("Escape is the one key that dismisses", () => {
    expect(["Escape", "Esc", "Enter", " ", "Tab"].map(dismisses)).toEqual([true, false, false, false, false]);
  });

  test("a dismissed hint is hidden and leaves hold-to-reveal", () => {
    expect(lookOf({ text: "x", exposable: true }, AT_REST)).toMatchObject({
      showing: "wanted",
      hint: { "data-exposable": "" },
    });
    expect(lookOf({ text: "x", exposable: true }, DISMISSED)).toMatchObject({
      showing: "dismissed",
      hint: { "data-exposable": undefined },
    });
  });

  test("a hint inside a form never answers hold-to-reveal", () => {
    expect(lookOf({ text: "x" }, AT_REST).hint["data-exposable"]).toBeUndefined();
  });
});
