// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The vendor picker fills the addresses the city answered with, and a
// face change follows only an address the picker filled.

import { describe, expect, test } from "bun:test";

import type { KnownHost } from "../../../wire";
import { followed, picked, rowFor } from "./known";

const DEEPSEEK: KnownHost = {
  host: "api.deepseek.com",
  faces: [
    { dialect: "open_ai", base_url: "https://api.deepseek.com" },
    { dialect: "open_ai_responses", base_url: "https://api.deepseek.com" },
    { dialect: "anthropic", base_url: "https://api.deepseek.com/anthropic/v1" },
  ],
};

describe("the vendor picker", () => {
  test("keeps the face already chosen when the host speaks it", () => {
    expect(picked(DEEPSEEK, "messages")).toEqual({
      wireApi: "messages",
      baseUrl: "https://api.deepseek.com/anthropic/v1",
    });
  });

  test("falls to the host's main face when it does not", () => {
    const anthropicOnly: KnownHost = {
      host: "api.anthropic.com",
      faces: [{ dialect: "anthropic", base_url: "https://api.anthropic.com/v1" }],
    };
    expect(picked(anthropicOnly, "chat")).toEqual({
      wireApi: "messages",
      baseUrl: "https://api.anthropic.com/v1",
    });
  });

  test("finds the row for a URL whatever its case", () => {
    expect(rowFor([DEEPSEEK], "https://API.DeepSeek.com/anthropic")?.host).toBe("api.deepseek.com");
    expect(rowFor([DEEPSEEK], "https://relay.example.test/v1")).toBeNull();
  });

  test("moves a filled address to the new face, and leaves a typed one alone", () => {
    expect(followed(DEEPSEEK, "https://api.deepseek.com", "messages")).toBe(
      "https://api.deepseek.com/anthropic/v1",
    );
    expect(followed(DEEPSEEK, "https://api.deepseek.com/beta", "messages")).toBeNull();
    expect(followed(DEEPSEEK, "https://api.deepseek.com", "responses")).toBeNull();
  });
});
