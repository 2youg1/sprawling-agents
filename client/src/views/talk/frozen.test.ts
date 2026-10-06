// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";
import { FIRST_POLICY } from "../../core/commands";
import { say } from "../../core/lang";
import { TimeMs } from "../../wire";
import { firstHeadSaid, frozenSaid } from "./frozen";
import { policyFace } from "./policy";

test("the first head keeps the opening write limit and trial policy", () => {
  const policy = { ...FIRST_POLICY, write: "create", admit: "tested", landing: "experiment" } as const;
  const opening = { at: TimeMs.make(1), task: "review", goal: "", effort: "high", policy } as const;
  expect(firstHeadSaid(opening, "en")).toBe([
    say("en", "effort_high"), say("en", `mode_${policy.mode}`), policyFace("en", policy),
  ].join(" · "));
  expect(frozenSaid(opening, "en")).toBe([say("en", "effort_high"), say("en", `mode_${policy.mode}`)].join(" · "));
  expect(firstHeadSaid({ at: opening.at, task: opening.task, goal: opening.goal }, "en")).toBe("");
});
