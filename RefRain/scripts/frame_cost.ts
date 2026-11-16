// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Result, Schema } from "effect";

import { decodeFrame } from "../src/core/frames";
import { ServerFrame } from "../src/wire";

const eventText = JSON.stringify({
  event: {
    data: { text: "hello there", n: 3 },
    kind: "tool_called",
    prev: "a".repeat(64),
    run: "01234567-89ab-cdef-0123-456789abcdef",
    seq: 42,
    t: 1700000000000,
    v: 1,
    who: "mayor",
  },
});
const bySchema = Schema.decodeUnknownResult(Schema.fromJsonString(ServerFrame));

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

const hot = floorMicros(() => decodeFrame(eventText));
const schema = floorMicros(() => Result.isSuccess(bySchema(eventText)));
console.log(
  `client_frame_decode event_frame hot=${hot.toFixed(2)}us schema=${schema.toFixed(2)}us`,
);
