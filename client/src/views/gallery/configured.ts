// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a made-up city answers about how it is set up, written once for
// every fixture that asks: an endpoint's tuning as it reads back, and
// the `[search]` part of the configuration answer.

import type { EndpointTuning, SettledSearch } from "../../wire";

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
