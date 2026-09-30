// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The instrument behind the `client_frame_decode` row of tools/xtask/budgets.toml:
// what the page pays to read one event frame off the socket, by the hot
// decoder and by the Effect schema it stands in for, interleaved on the
// same machine. It prints a reading and judges nothing, because a
// wall-clock figure is the machine's and a loaded machine is not a defect.
//
//   cd client && bun scripts/frame_cost.ts

import { Either, Schema } from "effect";

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
const bySchema = Schema.decodeUnknownEither(Schema.parseJson(ServerFrame));

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

const hot = floorMicros(() => decodeFrame(eventText));
const schema = floorMicros(() => Either.isRight(bySchema(eventText)));
console.log(
  `client_frame_decode event_frame hot=${hot.toFixed(2)}us schema=${schema.toFixed(2)}us`,
);
