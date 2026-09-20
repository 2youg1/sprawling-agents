// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { QUERIES, createAsking, keyOf } from "./asking";
import { Query } from "../wire";

// Every question the wire lets a page ask by name alone. The union is
// generated, so this reads the same table the city answers from.
function nullaryQueries(): readonly string[] {
  return Query.members.flatMap((member) =>
    "literals" in member
      ? member.literals.filter((value) => typeof value === "string")
      : [],
  );
}

describe("asking", () => {
  // An answer is filed by the question it settles. A question absent
  // from `QUERIES` matched nothing on arrival, so its answer was
  // dropped in silence and the page waited for ever: that is how
  // `toolkits`, `release` and `mcp_health` went blank.
  test("every question the wire takes by name alone has a row", () => {
    const rows: readonly string[] = Object.values(QUERIES);
    for (const name of nullaryQueries()) {
      expect(rows, `${name} has no row in QUERIES`).toContain(name);
    }
    expect(Object.keys(QUERIES)).toHaveLength(nullaryQueries().length);
  });

  test("an answer with no subject of its own settles the question by name", () => {
    const asked: string[] = [];
    const asking = createAsking((query) => {
      asked.push(keyOf(query));
      return true;
    });
    const shelf = asking.ask(QUERIES.toolkits);
    expect(asked).toEqual([keyOf(QUERIES.toolkits)]);
    expect(shelf()).toBeUndefined();

    asking.answered({ toolkits: "unenrolled" });
    expect(shelf()).toEqual({ toolkits: "unenrolled" });
  });

  test("a question stays unanswered until its own answer lands", () => {
    const asking = createAsking(() => true);
    const doctor = asking.ask(QUERIES.doctor);
    const city = asking.ask(QUERIES.city);

    asking.answered({
      city: { active: 0, buildings: [], frozen: 0, halted: [], pursuits: [], runs: [] },
    });
    expect(doctor()).toBeUndefined();
    expect(city()).toBeDefined();
  });
});
