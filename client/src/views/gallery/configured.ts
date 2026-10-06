// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a made-up city answers about how it is set up, written once for
// every fixture that asks: an endpoint's tuning as it reads back, and
// the `[search]` part of the configuration answer.

import type { EndpointSummary, EndpointTuning, SettledSearch } from "../../wire";
import { ServerLabel } from "../../wire";

// What an endpoint read back when nobody opened its advanced section:
// every figure absent, and the proxy rule the city settles absence to.
export const UNTUNED: EndpointTuning = { headers: [], overrides: [], proxying: "except_local" };

// `[search]` as a city that never wrote one answers it: the default
// supplier, stated by no file, at the address the city declares.
export const SEARCH: SettledSearch = {
  account_status: [],
  configuration: "default",
  default_url: "https://mcp.exa.ai/mcp",
  from: "default",
};

// One provider with three accounts in order, one for each state a key
// can be in that a person has to read differently: stored, missing,
// and read from an environment variable the page cannot write.
export const ACCOUNTED: EndpointSummary = {
  base_url: "https://api.zenmux.ai/v1",
  connection_kind: "openai_compat",
  dialect: "open_ai",
  has_credential: true,
  label: "ZenMux",
  local: false,
  models: [{ id: "fable", input_modalities: [] }],
  name: "zenmux",
  tuning: {
    ...UNTUNED,
    accounts: [
      { id: ServerLabel.make("main"), reference: "secret:providers/zenmux.main" },
      { id: ServerLabel.make("spare"), reference: "secret:providers/zenmux.spare" },
      { id: ServerLabel.make("team"), reference: "secret:providers/zenmux.team" },
    ],
  },
  account_status: [
    { id: ServerLabel.make("main"), key: "stored" },
    { id: ServerLabel.make("spare"), key: "missing" },
    { id: ServerLabel.make("team"), key: "environment" },
  ],
};

// The same provider as it stands when it was attached with one key and
// no list: the editor says what a first account replaces.
export const UNLISTED: EndpointSummary = { ...ACCOUNTED, account_status: [], tuning: UNTUNED };

// A city that lists two search services of its own, the second in use
// with one keyed account, while the hall's own file turns search off:
// the card edits the city's value and says the hall's stands apart.
export const SEARCH_CUSTOM: SettledSearch = {
  account_status: [{ supplier: ServerLabel.make("brave"), accounts: [{ id: ServerLabel.make("main"), key: "stored" }] }],
  city: {
    custom: {
      selected: ServerLabel.make("brave"),
      suppliers: [
        { id: ServerLabel.make("tavily"), url: "https://mcp.tavily.invalid/mcp", remote: "tavily_search", query_field: "query", accounts: [] },
        {
          id: ServerLabel.make("brave"),
          url: "https://mcp.brave.invalid/mcp",
          remote: "brave_web_search",
          query_field: "query",
          count_field: "count",
          accounts: [{ id: ServerLabel.make("main"), reference: "secret:search/brave.main", header: "x-subscription-token" }],
        },
      ],
    },
  },
  configuration: "off",
  default_url: "https://mcp.exa.ai/mcp",
  from: "building",
};
