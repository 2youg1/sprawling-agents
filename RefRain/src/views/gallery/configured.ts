// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { EndpointSummary, EndpointTuning, SettledSearch } from "../../wire";
import { ServerLabel } from "../../wire";

export const UNTUNED: EndpointTuning = { headers: [], overrides: [], proxying: "except_local" };

export const SEARCH: SettledSearch = {
  account_status: [],
  configuration: "default",
  default_url: "https://mcp.exa.ai/mcp",
  from: "default",
};

export const ACCOUNTED: EndpointSummary = {
  base_url: "https://api.example.test/v1",
  connection_kind: "openai_compat",
  dialect: "open_ai",
  has_credential: true,
  label: "Example",
  local: false,
  models: [
    {
      id: "fable",
      input_modalities: [],
      canonical: "fable",
      listing: "listed",
      thinking: { levels: [], on: "unknown", from: "unknown", words: [] },
      generation: {
        output_pointer: "/max_tokens",
        settings: {},
        temperature: { support: "unknown" },
        top_p: { support: "unknown" },
        top_k: { support: "unknown" },
      },
    },
  ],
  name: "example",
  tuning: {
    ...UNTUNED,
    accounts: [
      { id: ServerLabel.make("main"), reference: "secret:providers/example.main" },
      { id: ServerLabel.make("spare"), reference: "secret:providers/example.spare" },
      { id: ServerLabel.make("team"), reference: "secret:providers/example.team" },
    ],
  },
  account_status: [
    { id: ServerLabel.make("main"), key: "stored" },
    { id: ServerLabel.make("spare"), key: "missing" },
    { id: ServerLabel.make("team"), key: "environment" },
  ],
};

export const UNLISTED: EndpointSummary = { ...ACCOUNTED, account_status: [], tuning: UNTUNED };

export const SEARCH_CUSTOM: SettledSearch = {
  account_status: [
    {
      supplier: ServerLabel.make("brave"),
      accounts: [{ id: ServerLabel.make("main"), key: "stored" }],
    },
  ],
  city: {
    custom: {
      selected: ServerLabel.make("brave"),
      suppliers: [
        {
          id: ServerLabel.make("tavily"),
          url: "https://mcp.tavily.invalid/mcp",
          remote: "tavily_search",
          query_field: "query",
          accounts: [],
        },
        {
          id: ServerLabel.make("brave"),
          url: "https://mcp.brave.invalid/mcp",
          remote: "brave_web_search",
          query_field: "query",
          count_field: "count",
          accounts: [
            {
              id: ServerLabel.make("main"),
              reference: "secret:search/brave.main",
              header: "x-subscription-token",
            },
          ],
        },
      ],
    },
  },
  configuration: "off",
  default_url: "https://mcp.exa.ai/mcp",
  from: "building",
};
