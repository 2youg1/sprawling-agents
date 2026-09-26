// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import type { DoctorItem } from "../../wire";
import { absenceOf } from "./dependencies";

const card = (name: string, here: boolean): DoctorItem => ({
  name,
  install: "unknown_platform",
  need: "required",
  tier: "use",
  state: here
    ? { present: { at: name, version: "silent" } }
    : { absent: { absence: "not_on_search_path" } },
});

const drivers = [card("gecko", true), card("webkit", false), card("chromedriver", false)];

describe("absenceOf", () => {
  test("a driver the tier did without is a spare once the verdict names no group", () => {
    expect(drivers.map((each) => absenceOf(each, [], drivers))).toEqual(["wanted", "spare", "spare"]);
  });

  test("a card the verdict names, and every card of a group still short, are wanted", () => {
    const none = [card("webkit", false), card("git", false)];
    expect(none.map((each) => absenceOf(each, ["git", "browser"], none))).toEqual(["wanted", "wanted"]);
    expect(absenceOf(card("webkit", false), null, drivers)).toEqual("wanted");
  });
});
