// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A range the stream skipped is folded without invalidating anything,
// because the answers of the open socket were asked after it. Once that
// socket dies those answers die with it, so a reconnect inside the
// resume threshold turns the pending range into one whose records mark
// stale what they touch.

import { expect, test } from "bun:test";
import { Schema } from "effect";

import { createGapWalk } from "./gap_walk";
import { AskId, AskOutcome, EventRecord, Seq } from "../wire";
import type { Query } from "../wire";

function record(seq: number): EventRecord {
  return Schema.decodeUnknownSync(EventRecord)({
    seq,
    prev: "0".repeat(64),
    t: 1,
    v: 1,
    who: "city",
    run: "00000000-0000-4000-8000-000000000001",
    kind: "log_truncated",
    data: {},
  });
}

test("a lagged range pending at a reconnect invalidates what its records touch", () => {
  const asked: Query[] = [];
  const invalidated: EventRecord[] = [];
  const walk = createGapWalk(
    (query) => {
      asked.push(query);
      return AskId.make(asked.length);
    },
    { apply: () => null, refused: () => undefined, forget: () => undefined },
    { invalidate: (each) => invalidated.push(each), reconnected: () => undefined, resumed: () => undefined },
    "en",
  );
  walk.welcomed("epoch", null);
  walk.fold(record(10));
  walk.lagged(Seq.make(11), Seq.make(20));
  walk.welcomed("epoch", Seq.make(30));
  const page = Schema.decodeUnknownSync(AskOutcome)({ answer: { history_range: { from: 11, to: 20, records: [record(12)] } } });

  expect(walk.answered(AskId.make(asked.length), page)).toBe(true);
  expect(invalidated).toEqual([record(12)]);
});
