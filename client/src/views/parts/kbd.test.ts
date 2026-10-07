// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The chord sheet's wiring as client/Spec.lean §7-3 states it for every
// dialog: the heading names it, the line that says where chords are
// rebound describes it, and Escape is answered by the caller rather than
// by the element. A specimen on #/gallery is open in the page's flow; the
// modal seat is opened by `showModal()`, never by the attribute, because
// an `open` attribute makes a dialog that is not modal.

import { describe, expect, test } from "bun:test";

import { sheetOf, type Seat } from "./kbd";

const WORDS = {
  title: "Keyboard",
  dismiss: "Close",
  where: "Rebind a chord in Settings.",
  rows: [{ key: "help", label: "Show the chords", marks: ["Ctrl", "?"] }],
};

function sheet(seat: Seat, heard: string[]) {
  return sheetOf({ uid: "k1", seat, onClose: () => heard.push("close"), hold: () => undefined }, WORDS);
}

describe("the chord sheet's relations", () => {
  test("the heading names it and the rebinding line describes it", () => {
    const look = sheet("modal", []);
    expect([look.sheet["aria-labelledby"], look.sheet["aria-describedby"]]).toEqual(["k1-title", "k1-where"]);
    expect([look.heading, look.where]).toEqual([
      { id: "k1-title", text: "Keyboard" },
      { id: "k1-where", text: "Rebind a chord in Settings." },
    ]);
  });

  test("only a specimen carries the open attribute", () => {
    expect([sheet("modal", []).sheet.open, sheet("specimen", []).sheet.open]).toEqual([false, true]);
  });
});

describe("leaving the chord sheet", () => {
  test("Escape is refused to the element and handed to the caller, as the dismiss button is", () => {
    const heard: string[] = [];
    let refused = false;
    const look = sheet("modal", heard);
    look.sheet.oncancel?.({
      preventDefault: () => {
        refused = true;
      },
    });
    look.dismiss.onPress();
    expect(refused).toBe(true);
    expect(heard).toEqual(["close", "close"]);
  });
});
