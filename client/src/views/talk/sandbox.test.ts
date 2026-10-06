// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";
import { Address, Tokens, UsdMicros } from "../../wire";
import type { Answer, SandboxLimits } from "../../wire";
import { restriction, sandboxQuery } from "./sandbox";

const LIMITS: SandboxLimits = { fuel: 0, mounts: [Address.make("lab")], shell: true };

function building(sandbox: SandboxLimits | null): Answer {
  return {
    building: {
      addr: Address.make("lab"),
      archive: [],
      blocked: [],
      docs: [],
      mcp: [],
      plan: [],
      problems: [],
      progress: { unplanned: { budget: { tokens: Tokens.make(0), usd: UsdMicros.make(0) }, steps: 0 } },
      rooms: [],
      sandbox,
    },
  };
}

test("a sandbox is a fact only where it restricts a run", () => {
  expect([
    restriction(undefined),
    restriction(building(null)),
    restriction(building({ ...LIMITS, arm: "none" })),
    restriction(building(LIMITS)),
    restriction(building({ ...LIMITS, arm: "copied_tree" })),
  ]).toEqual([null, null, null, LIMITS, { ...LIMITS, arm: "copied_tree" }]);
});

test("a room's sandbox is asked of its building", () => {
  expect(sandboxQuery(Address.make("lab/fresh"))).toEqual({ building_view: { addr: Address.make("lab") } });
});
