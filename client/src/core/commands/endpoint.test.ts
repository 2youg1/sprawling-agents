// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The two commands an attached endpoint's model editor sends. They keep
// the city's tuning and credential archive, and never replay an account
// list that another disclosure may have changed since the editor opened.

import { describe, expect, test } from "bun:test";

import { ProviderName, ServerLabel } from "../../wire";
import type { EndpointSummary } from "../../wire";
import { probeAttached, saveModels } from "./endpoint";

const SUMMARY: EndpointSummary = {
  name: "house",
  label: "House",
  base_url: "https://gateway.example.invalid/v1",
  dialect: "open_ai",
  connection_kind: "openai_compat",
  models: [],
  local: false,
  has_credential: true,
  tuning: {
    timeout_ms: 45_000,
    headers: [{ name: "x-tenant", value: "blue" }],
    overrides: [{ pointer: "/temperature", value: "0" }],
    accounts: [{ id: ServerLabel.make("fresh"), reference: "secret:providers/house.fresh" }],
    account_retries: "two",
  },
  account_status: [],
};

describe("an attached endpoint's model commands", () => {
  test("refresh sends the complete stored tuning, including custom headers, and no account list", () => {
    expect(probeAttached(SUMMARY)).toMatchObject({
      probe_endpoint: {
        name: ProviderName.make("house"),
        base_url: SUMMARY.base_url,
        dialect: "open_ai",
        secret: null,
        auth_header: null,
        tuning: { ...SUMMARY.tuning, accounts: null },
      },
    });
  });

  test("save partitions listed and named ids, preserves the rest of tuning, and sends no account list", () => {
    expect(saveModels(SUMMARY, { listed: ["a", "c"], named: ["typed"] })).toMatchObject({
      attach_endpoint: {
        name: ProviderName.make("house"),
        base_url: SUMMARY.base_url,
        dialect: "open_ai",
        secret: null,
        auth_header: null,
        admit: ["a", "c"],
        named: ["typed"],
        tuning: { ...SUMMARY.tuning, accounts: null },
      },
    });
  });
});
