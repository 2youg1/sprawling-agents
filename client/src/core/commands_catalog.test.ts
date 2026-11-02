// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";

import { Address, B3Hash, ServerLabel } from "../wire";
import { configureCatalog } from "./commands";

const DIGEST = B3Hash.make("af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262");

test("one catalog save carries references, pins, positions and Reading Room names", () => {
  const command = configureCatalog(Address.make("lab"), {
    catalog: { tools: { exec: "dormant" }, namespaces: {}, skills: { how: "core" } },
    mcp_references: [
      { label: ServerLabel.make("docs"), definition: DIGEST, tools: { docs_read: DIGEST } },
    ],
    skill_pins: { how: DIGEST },
    reading_room: ["how"],
  });
  const sent = "configure_building" in command ? command.configure_building : undefined;
  expect(sent === undefined ? undefined : { ...sent, idem: "-" }).toEqual({
    addr: Address.make("lab"),
    sandbox: null,
    mcp: null,
    desktop: null,
    context_second_threshold: null,
    catalog: { tools: { exec: "dormant" }, namespaces: {}, skills: { how: "core" } },
    mcp_references: [
      { label: ServerLabel.make("docs"), definition: DIGEST, tools: { docs_read: DIGEST } },
    ],
    skill_pins: { how: DIGEST },
    reading_room: ["how"],
    idem: "-",
  });
  expect(JSON.stringify(command)).not.toContain("command");
  expect(JSON.stringify(command)).not.toContain("url");
});
