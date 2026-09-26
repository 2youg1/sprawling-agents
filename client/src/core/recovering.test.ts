// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { Option } from "effect";

import { formOf, recoveryFor } from "./recovering";
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

describe("the recovery table", () => {
  // A page and a city that disagree about the wire meet the same
  // disagreement on every reconnect: offering "try again" there ran the
  // person round a loop, and fetching the client this city was built
  // with is the one thing that ends it.
  test("a wire mismatch offers a reload, never a reconnect", () => {
    expect(recoveryFor({ code: "E_WIRE_MISMATCH", subject: "" }).map((recovery) => recovery.kind)).toEqual(["reload"]);
  });
});

// `/stop` cancels the run a refusal names and nothing wider, so it is
// offered only where the subject is a run: on a room it could never run.
describe("stopping from a notice", () => {
  test("a refusal about a run offers /stop", () => {
    expect(recoveryFor({ code: "E_BUSY", subject: "00000000-0000-4000-8000-000000000001" })).toEqual([
      { kind: "command", spelled: "/stop" },
    ]);
  });

  test("a refusal about a room offers nothing to stop", () => {
    expect(recoveryFor({ code: "E_BUSY", subject: "hall/room" })).toEqual([]);
  });
});

// A standing goal set on a building without a plan is refused, and the
// refusal offers the one form that gets the building a plan: the
// mayor's composer, filled with a request the person still sends.
describe("a standing goal without a plan", () => {
  test("offers a form that asks the mayor to write the plan", () => {
    expect(recoveryFor({ code: "E_PLAN_MISSING", subject: "hall: ship it" })).toEqual([
      { kind: "form", label: "act_ask_plan", words: "form_ask_plan", room: "mayor" },
    ]);
  });
});
