// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { selectModel } from "./commands";

// What a person states about a model reaches the frame, and what they
// did not state is absent rather than guessed (client-SPEC §5).
describe("choosing a model", () => {
  test("carries the input kinds the person stated", () => {
    const frame = selectModel("zenmux", "vision-1", "main", { contextTokens: null, maxOutputTokens: null, input: "text_image" });
    expect("select_model" in frame ? frame.select_model.input : undefined).toBe("text_image");
  });

  test("leaves the input kinds to the city when nobody stated them", () => {
    const frame = selectModel("zenmux", "plain-1", "main");
    expect("select_model" in frame ? frame.select_model.input : undefined).toBeNull();
  });
});
