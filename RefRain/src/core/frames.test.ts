// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";
import { Result, Schema } from "effect";

import { ServerFrame } from "../wire";
import { decodeFrame, inspectFrame } from "./frames";

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
  return Result.getOrNull(Schema.decodeUnknownResult(Schema.fromJsonString(ServerFrame))(text));
};

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

test("diagnoses syntax and the first invalid field without copying its input", () => {
  const secret = "sentinel-credential-not-for-diagnostics";
  const broken = inspectFrame(`{"delta":{"increment":{"said":"tok"},"run":"${secret}"}}`);
  expect(Result.isFailure(broken) && broken.failure).toMatchObject({
    stage: "schema",
    path: "delta.run",
    actual: "string",
  });
  expect(JSON.stringify(broken)).not.toContain(secret);
  const wrongType = inspectFrame(
    JSON.stringify({
      delta: { increment: { said: { value: secret } }, run: RUN },
    }),
  );
  expect(Result.isFailure(wrongType) && wrongType.failure).toMatchObject({
    stage: "schema",
    path: "delta.increment.said",
    actual: "object",
  });
  expect(JSON.stringify(wrongType)).not.toContain(secret);
  const incomplete = inspectFrame(`{"secret":"${secret}",`);
  expect(Result.isFailure(incomplete) && incomplete.failure).toMatchObject({ stage: "json" });
  expect(JSON.stringify(incomplete)).not.toContain(secret);
});
