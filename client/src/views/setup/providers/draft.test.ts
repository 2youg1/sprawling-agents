// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { BASE_URL, freshDraft, hostOf, tuningHints, tuningOf } from "./draft";
import type { TuningDefaults } from "../../../wire";

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

describe("the base_url shape", () => {
  test("compiles under the flag the browser compiles a pattern with", () => {
    expect(() => new RegExp(BASE_URL.source, "v")).not.toThrow();
  });

  test("names the same host with a hyphen, a port and a path", () => {
    expect(
      [
        "my-host.example:8080/v1",
        "http://127.0.0.1:8811/v1",
        "https://api.deepseek.com/anthropic/v1",
      ].map(hostOf),
    ).toEqual(["my-host.example", "127.0.0.1", "api.deepseek.com"]);
  });
});
