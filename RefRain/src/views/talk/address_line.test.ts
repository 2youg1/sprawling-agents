// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The address line's value: where a long address may be cut, what a
// screen reader hears, and what a press asks for, read without a look.

import { expect, test } from "bun:test";

import { Seq } from "../../wire";
import { addressLineOf } from "./address_line";

test("the building part may be cut and the room is kept whole, with the session's number after it", () => {
  let opened = 0;
  const look = addressLineOf(
    "en",
    "atlas-payments-reconciliation-service/ledger-close",
    Seq.make(23),
    () => {
      opened += 1;
    },
  );
  expect({ building: look.building, room: look.room, session: look.session }).toEqual({
    building: "atlas-payments-reconciliation-service/",
    room: "ledger-close",
    session: "#23",
  });
  expect(
    look.wire["aria-label"].startsWith("atlas-payments-reconciliation-service/ledger-close · #23"),
  ).toBe(true);
  expect(look.wire["aria-haspopup"]).toBe("dialog");
  look.wire.onclick();
  expect(opened).toBe(1);
});

test("a finger's press opens the palette for touch, and a click no press came before is a key's", () => {
  const arrivals: string[] = [];
  const look = addressLineOf("en", "hall/mayor", null, (arrival) => {
    arrivals.push(arrival);
  });
  const button = { focus: () => undefined };
  look.wire.onpointerdown({ currentTarget: button, pointerType: "touch" });
  look.wire.onclick();
  look.wire.onclick();
  look.wire.onpointerdown({ currentTarget: button, pointerType: "mouse" });
  look.wire.onclick();
  expect(arrivals).toEqual(["touch", "keys", "keys"]);
});

test("a room with no session yet draws no number, and an address of one segment has no building part", () => {
  const look = addressLineOf("zh", "atlas", null, () => undefined);
  expect({ building: look.building, room: look.room, session: look.session }).toEqual({
    building: "",
    room: "atlas",
    session: undefined,
  });
  expect(look.wire["aria-label"].startsWith("atlas：")).toBe(true);
});
