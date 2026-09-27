// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { stoppedAt } from "./probed";
import type { Reached, Staged } from "./probed";

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
