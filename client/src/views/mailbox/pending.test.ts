// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import type { Notice } from "../../core/belief";
import { TimeMs } from "../../wire";
import type { AxCode } from "../../wire";
import { stoppedOf } from "./pending";

function notice(code: AxCode, at: number): Notice {
  return {
    error: { code, action: "edit", subject: "hall/notes.md", nearby: [], retry: "no", recovery: "" },
    seen: false,
    at: TimeMs.make(at),
    key: `${code} hall/notes.md`,
    count: 1,
    about: null,
  };
}

describe("what the deciding section holds", () => {
  // The regression a person met: a run edited a file, a tool call was
  // refused and the model went on, and the refusal waited in deciding.
  test("a tool refusal the run goes on from is not pending", () => {
    const recovered = (
      ["E_PATH_NOT_FOUND", "E_INVALID_ARGS", "E_VERSION_CONFLICT", "E_TOOL_UNKNOWN", "E_GATE_DENIED"] as const
    ).map((code, at) => notice(code, at));
    expect(stoppedOf(recovered)).toEqual({ asks: [], failures: [] });
  });

  test("a door and a stopped city are pending, newest first, each in its own kind", () => {
    const door = notice("E_APPROVAL_PENDING", 1);
    const key = notice("E_CREDENTIAL_MISSING", 2);
    const budget = notice("E_BUDGET_EXHAUSTED", 3);
    const edit = notice("E_VERSION_CONFLICT", 4);
    expect(stoppedOf([door, key, budget, edit])).toEqual({ asks: [door], failures: [budget, key] });
  });
});
