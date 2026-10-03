// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import type { Answer } from "../wire";
import { readAnswer } from "./answered";

const document = (answer: Answer) => ("document" in answer ? answer.document : undefined);

describe("readAnswer", () => {
  test("names the question the city could not answer", () => {
    expect(readAnswer({ unavailable: { query: "Document(town/a.md)" } }, document)).toEqual({
      kind: "unavailable",
      query: "Document(town/a.md)",
      reason: null,
    });
  });

  test("names the variant that arrived in a slot that asked for another", () => {
    expect(readAnswer({ run: null }, document)).toEqual({ kind: "unavailable", query: "run", reason: null });
  });

  test("keeps the wait apart from the answer", () => {
    expect(readAnswer(undefined, document)).toEqual({ kind: "asking" });
  });
});
