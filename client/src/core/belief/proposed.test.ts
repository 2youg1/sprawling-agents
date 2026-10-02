// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Which documents the mailbox asks about (client/Spec.lean §4-55): the ones a
// `proposal_offered` line this page folded named, at the newest such
// line. Driven through the belief's own door, the way the socket and a
// gap walk fold records.

import { describe, expect, test } from "bun:test";
import { get } from "svelte/store";

import { createBelief } from "../belief";
import type { EventKind, EventRecord } from "../../wire";
import { Address, B3Hash, RunId, Seq, TimeMs } from "../../wire";

const RUN = RunId.make("11111111-1111-4111-8111-111111111111");
const PLAN = Address.make("shop/notes/plan.md");
const LOG = Address.make("shop/notes/log.txt");

function line(at: number, kind: EventKind, data: Record<string, unknown>): EventRecord {
  return {
    run: RUN,
    seq: Seq.make(at),
    kind,
    t: TimeMs.make(at),
    who: "shop/east",
    prev: B3Hash.make("0".repeat(64)),
    v: 1,
    data,
  };
}

function offered(at: number, doc: unknown): EventRecord {
  return line(at, "proposal_offered", {
    doc,
    baseline: "a".repeat(64),
    start: 0,
    end: 4,
    before: "Plan",
    after: "Notes",
  });
}

describe("the documents a card was offered on", () => {
  test("an offer names its document at the offer's position", () => {
    const store = createBelief(() => 0);
    store.apply(offered(4, PLAN));
    store.apply(offered(7, LOG));
    store.apply(offered(9, PLAN));
    expect(get(store.belief).proposed).toEqual(
      new Map([
        [PLAN, Seq.make(9)],
        [LOG, Seq.make(7)],
      ]),
    );
  });

  // A page that reconnects folds the newest records first and fills the
  // gap behind them afterwards; the older offer must not pull the
  // document back below the newer one.
  test("an older offer folded later leaves the newer position", () => {
    const store = createBelief(() => 0);
    store.apply(offered(9, PLAN));
    store.apply(offered(4, PLAN));
    expect(get(store.belief).proposed).toEqual(new Map([[PLAN, Seq.make(9)]]));
  });

  // The line still moves its run: the offer is a record of that run, and
  // a run whose position stood still would be asked about again.
  test("an offer whose document is not an address is reported and still folds its run", () => {
    const store = createBelief(() => 0);
    expect(store.apply(offered(5, "../outside the city"))).toBe("proposal_offered.doc");
    expect(get(store.belief).proposed).toEqual(new Map());
    expect(get(store.belief).runs[RUN]?.lastSeq).toBe(Seq.make(5));
  });

  // A decision names a card, not its document, so it cannot take one
  // off; the city's answer for the document says which cards remain.
  test("deciding or withdrawing a card leaves the document listed", () => {
    const store = createBelief(() => 0);
    store.apply(offered(4, PLAN));
    store.apply(line(5, "proposal_decided", { proposal: "b".repeat(64), verdicts: [] }));
    store.apply(line(6, "proposal_withdrawn", { proposal: "c".repeat(64) }));
    expect(get(store.belief).proposed).toEqual(new Map([[PLAN, Seq.make(4)]]));
  });

  test("another ledger forgets them", () => {
    const store = createBelief(() => 0);
    store.apply(offered(4, PLAN));
    store.forget();
    expect(get(store.belief).proposed).toEqual(new Map());
  });
});
