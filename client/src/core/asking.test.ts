// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { afterEach, describe, expect, test } from "bun:test";
import { get } from "svelte/store";

import { HELD_CAP, QUERIES, createAsking, keyOf } from "./asking";
import type { Reported } from "./asking";
import type { Key } from "./lang";
import { Address, AskId, B3Hash, Query, RunId, Seq, TimeMs } from "../wire";
import type { Answer, AskOutcome, EventRecord } from "../wire";

// Every question the wire lets a page ask by name alone. The union is
// generated, so this reads the same table the city answers from.
function nullaryQueries(): readonly string[] {
  return Query.members.flatMap((member) => {
    if ("literals" in member) return member.literals.filter((value) => typeof value === "string");
    if ("literal" in member && typeof member.literal === "string") return [member.literal];
    return [];
  });
}

// A clock the test moves, and the timers the patience books against it.
// The module reads neither global itself, which is the whole point of
// handing it a clock: a deadline nobody can drive is a deadline nobody
// has checked.
interface Driven {
  readonly ask: ReturnType<typeof createAsking>;
  readonly reports: [Key, Reported][];
  readonly sent: string[];
  // The id of each question that went out, in send order.
  readonly ids: AskId[];
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
  const ids: AskId[] = [];
  const ask = createAsking(
    (query) => {
      if (!send(query)) return null;
      sent.push(keyOf(query));
      const askId = AskId.make(ids.length + 1);
      ids.push(askId);
      return askId;
    },
    () => at,
    (phrase, error) => reports.push([phrase, error]),
  );
  return {
    ask,
    reports,
    sent,
    ids,
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

const answer = (value: Answer): AskOutcome => ({ answer: value });

// The id the n-th question went out under. The driver mints from 1, so
// a question that never went out answers under 0 and lands nowhere.
function nth(driver: Driven, at: number): AskId {
  return driver.ids[at] ?? AskId.make(0);
}

function record(at: number): EventRecord {
  return {
    run: RunId.make("00000000-0000-0000-0000-000000000000"),
    seq: Seq.make(at),
    kind: "run_started",
    t: TimeMs.make(at),
    who: "hall/mayor",
    prev: B3Hash.make("0".repeat(64)),
    v: 1,
    data: {},
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

  test("an answer settles the question that went out under its id", () => {
    const driver = driven();
    const shelf = driver.ask.ask(QUERIES.toolkits);
    expect(driver.sent).toEqual([keyOf(QUERIES.toolkits)]);
    expect(get(shelf)).toBeUndefined();

    driver.ask.answered(nth(driver, 0), Seq.make(1), answer({ toolkits: "unenrolled" }));
    expect(get(shelf)).toEqual({ toolkits: "unenrolled" });
  });

  test("a question stays unanswered until its own answer lands", () => {
    const driver = driven();
    const doctor = driver.ask.ask(QUERIES.doctor);
    const city = driver.ask.ask(QUERIES.city);

    driver.ask.answered(nth(driver, 1), Seq.make(1), answer({
      city: { active: 0, buildings: [], frozen: 0, halted: [], pursuits: [], runs: [] },
    }));
    expect(get(doctor)).toBeUndefined();
    expect(get(city)).toBeDefined();
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
    expect(get(doctor), "an answer already on screen would stay").toBeUndefined();
    expect(driver.reports).toHaveLength(1);
    expect(driver.reports[0]?.[0], "the sentence a person reads").toBe("ask_late");
    expect(driver.reports[0]?.[1].code).toBe("E_TIMEOUT");
    expect(driver.reports[0]?.[1].subject).toBe(keyOf(QUERIES.doctor));
  });

  // The defect: the privacy answer reads every control on the host and
  // took longer than the common patience, so every visit to the page
  // reported the city late and asked the whole read a second time. A
  // fast question asked behind it is still judged at its own deadline.
  test("a question that reads the host gets its own patience, and a fast one behind it keeps the common one", () => {
    const driver = driven();
    driver.ask.ask(QUERIES.privacy);
    driver.pass(1_000);
    driver.ask.ask(QUERIES.doctor);

    driver.pass(16_000);
    expect(driver.reports.map(([, error]) => error.subject), "only the fast question is late").toEqual([keyOf(QUERIES.doctor)]);
    expect(driver.sent, "the host is not asked to read again").toHaveLength(2);

    driver.pass(110_000);
    expect(driver.reports.map(([, error]) => error.subject)).toEqual([keyOf(QUERIES.doctor), keyOf(QUERIES.privacy)]);
  });

  // The late answer is still the answer: a question whose patience ran
  // out has left the queue, so the answer lands on the slot that asked
  // for it rather than being reported a second time as unplaceable.
  test("an answer that arrives after the patience still settles", () => {
    const driver = driven();
    const toolkits = driver.ask.ask(QUERIES.toolkits);
    driver.pass(16_000);
    expect(driver.reports).toHaveLength(1);

    driver.ask.answered(nth(driver, 0), Seq.make(1), answer({ toolkits: "unenrolled" }));
    expect(get(toolkits)).toEqual({ toolkits: "unenrolled" });
    expect(driver.reports, "a late answer is not an unplaceable one").toHaveLength(1);
  });

  // Two rounds of matching, both missed. Nothing a person can do about
  // an answer no question waits for, and it is not a protocol mismatch:
  // a page and a city from one build said "the versions differ" while
  // they were connected. The question that still waits is what a
  // person meets, once, when its patience runs out.
  test("an answer that matches no question is not a refusal", () => {
    const driver = driven();
    driver.ask.ask(QUERIES.doctor);

    driver.ask.answered(AskId.make(99), Seq.make(1), answer({ mcp_health: { addr: Address.make("hall/mayor"), servers: [] } }));
    expect(driver.reports).toEqual([]);

    driver.pass(16_000);
    expect(driver.reports.map(([phrase, error]) => [phrase, error.code])).toEqual([["ask_late", "E_TIMEOUT"]]);
  });

  // A page somebody is looking at asks again after the patience runs
  // out, so the screen fills itself the moment the city comes back.
  // The report is not repeated with it: the corner would otherwise
  // pop the same alert every fifteen seconds, and the second one
  // tells a person nothing the first did not.
  test("a watched question is asked again, and reported once", () => {
    const driver = driven();
    const watching = driver.ask.ask(QUERIES.doctor).subscribe(() => undefined);
    expect(driver.sent).toHaveLength(1);

    driver.pass(16_000);
    expect(driver.reports).toHaveLength(1);
    driver.pass(300);
    expect(driver.sent, "asked again while somebody is watching").toHaveLength(2);

    driver.pass(16_000);
    expect(driver.sent).toHaveLength(2);
    expect(driver.reports, "said once for one stretch of silence").toHaveLength(1);
    watching();
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

  // A long-lived tab visits more buildings than it keeps on screen, so
  // what it holds is bounded: past the cap the answer used least
  // recently and watched by nobody goes, and asking it again goes out.
  test("past the cap the least recently used unwatched answer is dropped", () => {
    const driver = driven();
    const health = (at: number): Query => ({ mcp_health: { addr: Address.make(`hall/a${String(at)}`) } });
    const fill = (from: number): void => {
      for (let at = from; at <= HELD_CAP; at += 1) {
        driver.ask.ask(health(at));
        driver.ask.answered(nth(driver, at), Seq.make(1), answer({ mcp_health: { addr: Address.make(`hall/a${String(at)}`), servers: [] } }));
      }
    };
    const watching = driver.ask.ask(health(0)).subscribe(() => undefined);
    driver.ask.answered(nth(driver, 0), Seq.make(1), answer({ mcp_health: { addr: Address.make("hall/a0"), servers: [] } }));
    fill(1);
    const sent = driver.sent.length;

    driver.ask.ask(health(0));
    expect(driver.sent, "a watched answer is never dropped").toHaveLength(sent);
    driver.ask.ask(health(1));
    expect(driver.sent, "the least recently used one was").toHaveLength(sent + 1);
    watching();
  });

  // The answer names the ledger position it was read at, so a record it
  // already holds - one that arrived while the question was out, or one
  // at or before that position afterwards - does not ask the city again.
  test("a record the answer already holds marks nothing stale", () => {
    const driver = driven();
    const watching = driver.ask.ask(QUERIES.city).subscribe(() => undefined);
    driver.ask.invalidate(record(5));
    driver.ask.answered(nth(driver, 0), Seq.make(7), answer({
      city: { active: 0, buildings: [], frozen: 0, halted: [], pursuits: [], runs: [] },
    }));
    driver.ask.invalidate(record(6));
    driver.pass(300);
    expect(driver.sent, "records 5 and 6 are inside the answer").toHaveLength(1);

    driver.ask.invalidate(record(7));
    driver.pass(300);
    expect(driver.sent, "record 7 is news").toHaveLength(2);
    watching();
  });

  // A view that has folded nothing answers with the first seq as the next
  // one it has not folded, so the genesis record that lands afterwards is
  // news rather than something the answer already holds.
  test("an answer read before genesis is stale once genesis lands", () => {
    const driver = driven();
    const watching = driver.ask.ask(QUERIES.city).subscribe(() => undefined);
    driver.ask.answered(nth(driver, 0), Seq.make(0), answer({
      city: { active: 0, buildings: [], frozen: 0, halted: [], pursuits: [], runs: [] },
    }));
    driver.ask.invalidate(record(0));
    driver.pass(300);
    expect(driver.sent, "the genesis record is news").toHaveLength(2);
    watching();
  });
});
