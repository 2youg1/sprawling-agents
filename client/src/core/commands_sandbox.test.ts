// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { Address, EnvVarName, ServerLabel } from "../wire";
import type { SandboxLimits } from "../wire";
import { configureSandbox } from "./commands";

// `ConfigureBuilding` carries four faces of a building in one verb, and
// the city reads `null` as "leave this face as it is". The sandbox card
// must therefore speak about the sandbox and nothing else, or saving it
// would wipe the building's MCP servers, desktop allowlist or context
// rung (client/Spec.lean §4-50).
describe("configureSandbox", () => {
  test("states the sandbox whole and leaves the other three faces alone", () => {
    const limits: SandboxLimits = {
      shell: true,
      fuel: 2_000_000,
      mounts: [Address.make("lab/shared")],
      env_passthrough: [EnvVarName.make("CC")],
      trusted: [ServerLabel.make("desktop")],
    };
    const command = configureSandbox(Address.make("lab"), limits);
    const sent = "configure_building" in command ? command.configure_building : undefined;
    expect(sent === undefined ? undefined : { ...sent, idem: "-" }).toEqual({
      addr: Address.make("lab"),
      sandbox: limits,
      mcp: null,
      desktop: null,
      context_second_threshold: null,
      idem: "-",
    });
  });
});
