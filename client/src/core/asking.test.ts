// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { afterEach, describe, expect, test } from "bun:test";
import { createRoot } from "solid-js";

import { QUERIES, createAsking, keyOf } from "./asking";
import type { Reported } from "./asking";
import type { Key } from "./lang";
import { Address, Query } from "../wire";

// Every question the wire lets a page ask by name alone. The union is
// generated, so this reads the same table the city answers from.
function nullaryQueries(): readonly string[] {
  return Query.members.flatMap((member) =>
    "literals" in member
      ? member.literals.filter((value) => typeof value === "string")
      : [],
  );
}

// A clock the test moves, and the timers the patience books against it.
// The module reads neither global itself, which is the whole point of
// handing it a clock: a deadline nobody can drive is a deadline nobody
// has checked.
interface Driven {
  readonly ask: ReturnType<typeof createAsking>;
  readonly reports: [Key, Reported][];
  readonly sent: string[];
  readonly pass: (ms: number) => void;
}

const real = globalThis.setTimeout;

afterEach(() => {
  Object.assign(globalThis, { setTimeout: real });
});

function driven(send: (query: Query) => boolean = () => true): Driven {
  let at = 1_000;
  const booked: { run: () => void; due: number }[] = [];
  Object.assign(globalThis, {
    setTimeout: (run: () => void, ms: number): number => {
      booked.push({ run, due: at + ms });
      return booked.length - 1;
    },
  });
  const reports: [Key, Reported][] = [];
  const sent: string[] = [];
  const ask = createAsking(
    (query) => {
      const went = send(query);
      if (went) sent.push(keyOf(query));
      return went;
    },
    () => at,
    (phrase, error) => reports.push([phrase, error]),
  );
  return {
    ask,
    reports,
    sent,
    // Time passes, then every timer that was due in it fires once, in
    // the order it was booked - which is how a browser would do it.
    pass: (ms: number) => {
      at += ms;
      const due = booked.filter((timer) => timer.due <= at);
      for (const timer of due) {
        booked.splice(booked.indexOf(timer), 1);
        timer.run();
      }
    },
  };
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
    const asking = createAsking(
      (query) => {
        asked.push(keyOf(query));
        return true;
      },
      () => 0,
      () => undefined,
    );
    const shelf = asking.ask(QUERIES.toolkits);
    expect(asked).toEqual([keyOf(QUERIES.toolkits)]);
    expect(shelf()).toBeUndefined();

    asking.answered({ toolkits: "unenrolled" });
    expect(shelf()).toEqual({ toolkits: "unenrolled" });
  });

  test("a question stays unanswered until its own answer lands", () => {
    const asking = createAsking(
      () => true,
      () => 0,
      () => undefined,
    );
    const doctor = asking.ask(QUERIES.doctor);
    const city = asking.ask(QUERIES.city);

    asking.answered({
      city: { active: 0, buildings: [], frozen: 0, halted: [], pursuits: [], runs: [] },
    });
    expect(doctor()).toBeUndefined();
    expect(city()).toBeDefined();
  });

  // The defect: a question the city never answered left `inflight`
  // true for the life of the tab, so the page drew a skeleton over an
  // answer that was never coming and nothing said why.
  test("a question nobody answers is reported", () => {
    const driver = driven();
    const doctor = driver.ask.ask(QUERIES.doctor);

    driver.pass(14_000);
    expect(driver.reports, "still inside its patience").toHaveLength(0);

    driver.pass(2_000);
    expect(doctor(), "an answer already on screen would stay").toBeUndefined();
    expect(driver.reports).toHaveLength(1);
    expect(driver.reports[0]?.[0], "the sentence a person reads").toBe("ask_late");
    expect(driver.reports[0]?.[1].code).toBe("E_TIMEOUT");
    expect(driver.reports[0]?.[1].subject).toBe(keyOf(QUERIES.doctor));
  });

  // The late answer is still the answer: a question whose patience ran
  // out has left the queue, so the answer lands on the slot that asked
  // for it rather than being reported a second time as unplaceable.
  test("an answer that arrives after the patience still settles", () => {
    const driver = driven();
    const toolkits = driver.ask.ask(QUERIES.toolkits);
    driver.pass(16_000);
    expect(driver.reports).toHaveLength(1);

    driver.ask.answered({ toolkits: "unenrolled" });
    expect(toolkits()).toEqual({ toolkits: "unenrolled" });
    expect(driver.reports, "a late answer is not an unplaceable one").toHaveLength(1);
  });

  // Two rounds of matching, both missed: before this the answer was
  // dropped without a word, which is the silence a page waiting for
  // ever is made of.
  test("an answer that matches no question is reported", () => {
    const driver = driven();
    driver.ask.ask(QUERIES.doctor);

    driver.ask.answered({ mcp_health: { addr: Address.make("hall/mayor"), servers: [] } });
    expect(driver.reports).toHaveLength(1);
    expect(driver.reports[0]?.[0]).toBe("ask_unfiled");
    expect(driver.reports[0]?.[1].code).toBe("E_WIRE_MISMATCH");
  });

  // A page somebody is looking at asks again after the patience runs
  // out, so the screen fills itself the moment the city comes back.
  // The report is not repeated with it: the corner would otherwise
  // pop the same alert every fifteen seconds, and the second one
  // tells a person nothing the first did not.
  test("a watched question is asked again, and reported once", () => {
    const driver = driven();
    createRoot(() => driver.ask.ask(QUERIES.doctor));
    expect(driver.sent).toHaveLength(1);

    driver.pass(16_000);
    expect(driver.reports).toHaveLength(1);
    driver.pass(300);
    expect(driver.sent, "asked again while somebody is watching").toHaveLength(2);

    driver.pass(16_000);
    expect(driver.sent).toHaveLength(2);
    expect(driver.reports, "said once for one stretch of silence").toHaveLength(1);
  });

  // A question that was never sent is not a question waiting on an
  // answer: the link was down, the slot is stale, and the patience has
  // nothing to run out on.
  test("a question the link refused is not reported as late", () => {
    const driver = driven(() => false);
    driver.ask.ask(QUERIES.doctor);

    driver.pass(30_000);
    expect(driver.reports).toHaveLength(0);
  });
});
