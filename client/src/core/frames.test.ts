// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";
import { Either, Schema } from "effect";

import { ServerFrame } from "../wire";
import { decodeFrame } from "./frames";

const RUN = "01234567-89ab-cdef-0123-456789abcdef";
const eventText = JSON.stringify({
  event: {
    data: { text: "hello there", n: 3 },
    kind: "tool_called",
    prev: "a".repeat(64),
    run: RUN,
    seq: 42,
    t: 1700000000000,
    v: 1,
    who: "mayor",
  },
});
const deltaText = JSON.stringify({
  delta: { increment: { said: "tok" }, run: RUN },
});

const bySchema = (text: string): ServerFrame | null => {
  const read = Schema.decodeUnknownEither(Schema.parseJson(ServerFrame))(text);
  return Either.isRight(read) ? read.right : null;
};

// The fastest of several batches: the floor the decoder reaches, which a
// busy neighbour on the machine can slow but not lower.
function floorMicros(run: () => unknown): number {
  const batch = 2000;
  let best = Number.POSITIVE_INFINITY;
  for (let round = 0; round < 15; round += 1) {
    const start = performance.now();
    for (let i = 0; i < batch; i += 1) run();
    best = Math.min(best, ((performance.now() - start) * 1000) / batch);
  }
  return best;
}

test("an event frame decodes within three microseconds", () => {
  expect(floorMicros(() => decodeFrame(eventText))).toBeLessThanOrEqual(3);
});

test("the hot frames read exactly what the schema reads", () => {
  expect(decodeFrame(eventText)).toEqual(bySchema(eventText));
  expect(decodeFrame(deltaText)).toEqual(bySchema(deltaText));
});

test("a hot frame the schema refuses is refused", () => {
  const badRun = eventText.replace(RUN, RUN.toUpperCase());
  const badKind = eventText.replace("tool_called", "tool_calld");
  const badDelta = deltaText.replace("said", "sang");
  for (const text of [badRun, badKind, badDelta, "{", "[]"]) {
    expect(decodeFrame(text)).toBeNull();
  }
});
