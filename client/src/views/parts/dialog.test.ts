// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The dialog's wiring as client/Spec.lean §7-3 states it: `aria-labelledby`
// on the heading, `aria-describedby` only when there is a detail, Escape
// answered by the caller rather than by the element, and the way out
// written before the way through, so the platform focuses it first.

import { describe, expect, test } from "bun:test";

import { lookOf, type DialogProps } from "./dialog";

function asked(over: Partial<DialogProps>, heard: string[]): DialogProps {
  return {
    open: true,
    title: "Remove this endpoint?",
    confirmLabel: "Delete",
    cancelLabel: "Cancel",
    onConfirm: () => heard.push("confirm"),
    onCancel: () => heard.push("cancel"),
    ...over,
  };
}

const HOLD = () => undefined;

describe("the relations on the dialog", () => {
  test("the heading names it, and a detail describes it only when there is one", () => {
    const bare = lookOf(asked({}, []), "s2", HOLD);
    expect(bare.sheet["aria-labelledby"]).toBe("s2-title");
    expect(bare.sheet["aria-describedby"]).toBeUndefined();
    expect(bare.detail).toBeUndefined();

    const told = lookOf(asked({ detail: "It cannot be undone." }, []), "s2", HOLD);
    expect(told.sheet["aria-describedby"]).toBe("s2-detail");
    expect(told.detail).toEqual({ id: "s2-detail", text: "It cannot be undone." });
    expect(told.heading).toEqual({ id: "s2-title", text: "Remove this endpoint?" });
  });

  test("Escape is refused to the element and handed to the caller", () => {
    const heard: string[] = [];
    let refused = false;
    lookOf(asked({}, heard), "s2", HOLD).sheet.oncancel?.({
      preventDefault: () => {
        refused = true;
      },
    });
    expect(refused).toBe(true);
    expect(heard).toEqual(["cancel"]);
  });
});

describe("the two answers", () => {
  test("the way out comes first, so it is the control under the hand", () => {
    const heard: string[] = [];
    const [first, second] = lookOf(asked({ destructive: true }, heard), "s2", HOLD).answers;
    expect([first.label, first.tone, second.label, second.tone]).toEqual([
      "Cancel",
      "secondary",
      "Delete",
      "destructive",
    ]);
    first.onPress();
    second.onPress();
    expect(heard).toEqual(["cancel", "confirm"]);
  });

  test("a confirmation that destroys nothing is the primary answer", () => {
    expect(lookOf(asked({}, []), "s2", HOLD).answers[1].tone).toBe("primary");
  });
});
