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

import { freshDraft, tuningHints, tuningOf } from "./draft";
import type { TuningDefaults } from "../../../wire";

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

// The boxes the person left empty show the figure the city would use,
// and that figure has to be the one the city derives: an idle timeout
// nobody set falls back to the call timeout, the typed one when the box
// holds it (`crates/gateway/src/endpoint/stream.rs`).
describe("the figures an empty tuning box shows", () => {
  const DEFAULTS: TuningDefaults = {
    from: "default",
    proxying: "except_local",
    timeout_ms: 120_000,
    request_max_retries: null,
    stream_idle_timeout_ms: null,
    account_retries: "two",
  };

  test("an untouched form shows the city's timeout for the idle box too", () => {
    expect(tuningHints(freshDraft(), DEFAULTS, "until halted")).toEqual({
      timeoutMs: "120000",
      requestRetries: "until halted",
      streamIdleMs: "120000",
    });
  });

  test("a typed call timeout is what the idle box falls back to", () => {
    expect(tuningHints({ ...freshDraft(), timeoutMs: "30000" }, DEFAULTS, "until halted")).toEqual({
      timeoutMs: "120000",
      requestRetries: "until halted",
      streamIdleMs: "30000",
    });
  });
});
