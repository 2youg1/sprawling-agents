// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { Option } from "effect";

import { formOf } from "./recovering";
import { MAYOR } from "./route";
import { Address } from "../wire";

const WORDS = "write a plan for {name} in {building}";

describe("a form recovery", () => {
  test("fills the named room's draft from the building and the name the subject carries", () => {
    expect(formOf("mayor", "district/east: nightly-report", WORDS)).toEqual(
      Option.some({ room: MAYOR, draft: "write a plan for nightly-report in district/east" }),
    );
    expect(formOf("building", "district/east: nightly-report", WORDS)).toEqual(
      Option.some({
        room: Address.make("district/east"),
        draft: "write a plan for nightly-report in district/east",
      }),
    );
  });

  // A subject without both parts opens nothing rather than a form
  // filled with a guess.
  test("opens nothing when the subject does not carry both parts", () => {
    for (const subject of ["district/east", "district/east:  ", ": nightly-report", "a/b : x"]) {
      expect(formOf("mayor", subject, WORDS), subject).toEqual(Option.none());
    }
  });
});
