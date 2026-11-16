// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { normalisedFrom, stoppedAt } from "./probed";
import type { Probed, Reached, Sent, Staged } from "./probed";

const PASSED: Staged = { state: "ok", detail: null, figure: null };

function answeredWith(status: number): Reached {
  return {
    host: "api.example.test",
    named: PASSED,
    connected: PASSED,
    answered: { state: "status", detail: null, figure: status },
    through: PASSED,
    elapsedMs: 90,
  };
}

describe("what a 401 on the unauthenticated reading says", () => {
  test("the host asked for a key and the list read with the key: reachable", () => {
    expect(stoppedAt(answeredWith(401), null)).toBe("setup_reach_ok");
  });

  test("the host asked for a key and the list refused it: the key", () => {
    expect(stoppedAt(answeredWith(403), { code: "E_PROVIDER", subject: "401 Unauthorized" })).toBe(
      "setup_reach_key",
    );
  });
});

// The line under the base_url box names the URL the city calls, and
// only for the probe this form sent, with the text and the face it was
// sent with, answered after it left. The box itself is the person's.
describe("the URL the city calls, under the box", () => {
  const ROOT = "https://api.deepseek.com";
  const ANTHROPIC = "https://api.deepseek.com/anthropic/v1";

  function answer(baseUrl: string): Probed {
    return { name: "deepseek", baseUrl, models: [], facts: [], reach: null, failure: null };
  }

  test("a probe under messages, then chat: the box keeps the address the face moved it to, with no line", () => {
    const pending: Sent = { id: "deepseek", baseUrl: ANTHROPIC, wireApi: "messages", before: null };
    expect(
      normalisedFrom(answer(ANTHROPIC), pending, { baseUrl: ROOT, wireApi: "chat" }),
    ).toBeNull();
  });

  test("a probe, then a typed path: the typed path stands, with no line", () => {
    const pending: Sent = { id: "deepseek", baseUrl: ROOT, wireApi: "chat", before: null };
    expect(
      normalisedFrom(answer(ROOT), pending, { baseUrl: `${ROOT}/v2`, wireApi: "chat" }),
    ).toBeNull();
  });

  test("a second probe does not act on the answer that stood when it left, nor on another endpoint's", () => {
    const standing = answer(ANTHROPIC);
    const pending: Sent = { id: "deepseek", baseUrl: ROOT, wireApi: "chat", before: standing };
    const now = { baseUrl: ROOT, wireApi: "chat" } as const;
    const elsewhere: Probed = { ...answer(`${ROOT}/v1`), name: "openai" };
    expect([
      normalisedFrom(standing, pending, now),
      normalisedFrom(elsewhere, pending, now),
      normalisedFrom(answer(`${ROOT}/v1`), pending, now),
    ]).toEqual([null, null, `${ROOT}/v1`]);
  });

  test("an answer that recorded another spelling shows the line until the text or the face moves", () => {
    const pending: Sent = {
      id: "deepseek",
      baseUrl: "api.deepseek.com",
      wireApi: "chat",
      before: null,
    };
    const recorded = answer(ROOT);
    expect([
      normalisedFrom(recorded, pending, { baseUrl: "api.deepseek.com", wireApi: "chat" }),
      normalisedFrom(recorded, pending, { baseUrl: "api.deepseek.co", wireApi: "chat" }),
      normalisedFrom(recorded, pending, { baseUrl: "api.deepseek.com", wireApi: "responses" }),
    ]).toEqual([ROOT, null, null]);
  });
});
