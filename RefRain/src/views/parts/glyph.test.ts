// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { statusLook, type Status } from "./glyph";

const FIVE: Record<Status, Status> = {
  idle: "idle",
  live: "live",
  waiting: "waiting",
  refused: "refused",
  done: "done",
};

describe("the five states each have their own drawing", () => {
  test("no two states are told apart by colour alone", () => {
    const drawings = Object.values(FIVE).map((state) => statusLook(state).glyph);
    expect(new Set(drawings).size).toBe(drawings.length);
  });
});
