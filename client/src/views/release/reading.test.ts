// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import type { AxError, InstallChannel, RegistryNewest } from "../../wire";
import { npmNewest, registryLineOf, updateOf } from "./reading";

const NEWEST = { version: "0.0.9", released: "2026-10-02" };

const refusal: AxError = {
  action: "read the newest release",
  code: "E_MODEL_UNCHOSEN",
  nearby: [],
  recovery: "check the network",
  retry: "no",
  subject: "registry.npmjs.org",
};

describe("registryLineOf", () => {
  test("a reading, a refusal and a registry not asked each say their own line", () => {
    const lines: RegistryNewest[] = [
      { registry: "npm", reading: { read: { newest: NEWEST } } },
      { registry: "crates_io", reading: "unasked" },
      { registry: "npm", reading: { refused: { refusal } } },
    ];
    expect(lines.map(registryLineOf)).toEqual([
      { registry: "release_registry_npm", newest: NEWEST, reason: null },
      { registry: "release_registry_crates_io", newest: null, reason: { key: "release_registry_unasked", said: null } },
      { registry: "release_registry_npm", newest: null, reason: { key: "release_registry_refused", said: "check the network" } },
    ]);
  });

  test("the verdict's version is npm's, whatever crates.io read", () => {
    expect(
      npmNewest([
        { registry: "crates_io", reading: { read: { newest: { version: "9.9.9", released: "x" } } } },
        { registry: "npm", reading: { read: { newest: NEWEST } } },
      ]),
    ).toEqual(NEWEST);
    expect(npmNewest([{ registry: "npm", reading: "unasked" }])).toBeNull();
  });
});

describe("updateOf", () => {
  test("every channel has its sentence, and the command is the city's", () => {
    const channels: InstallChannel[] = ["npm", "cargo", "archive", "source"];
    expect(channels.map((channel) => updateOf({ channel, command: channel === "source" ? null : `update ${channel}` }))).toEqual([
      { channel: "release_channel_npm", command: "update npm" },
      { channel: "release_channel_cargo", command: "update cargo" },
      { channel: "release_channel_archive", command: "update archive" },
      { channel: "release_channel_source", command: null },
    ]);
    expect(updateOf({ channel: "npm" })).toEqual({ channel: "release_channel_npm", command: null });
  });
});
