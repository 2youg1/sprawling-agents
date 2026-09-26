// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The endpoint form's tuning figures. The city holds the three numbers
// an untuned endpoint is called with (`gateway::EndpointTuning::DEFAULTS`)
// and says them through `Query::Config`; a form that sends nothing for
// a box nobody touched lets the city apply them, and a form that
// pre-filled its own numbers attached a differently tuned endpoint
// while the person had filled in nothing.

import { describe, expect, test } from "bun:test";

import { freshDraft, tuningOf } from "./draft";

// Bun's `navigator` carries no language, and the preference record a
// fresh draft reads its proxy rule from is opened with one.
Object.defineProperty(navigator, "language", { value: "en", configurable: true });

describe("an untouched endpoint form", () => {
  test("attaches with the city's own tuning", () => {
    const { timeoutMs, requestMaxRetries, streamIdleTimeoutMs } = tuningOf(freshDraft());
    expect({ timeoutMs, requestMaxRetries, streamIdleTimeoutMs }).toEqual({
      timeoutMs: null,
      requestMaxRetries: null,
      streamIdleTimeoutMs: null,
    });
  });
});
