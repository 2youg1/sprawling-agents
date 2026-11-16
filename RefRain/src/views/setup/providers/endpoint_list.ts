// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { wireApiOf } from "../../../core/commands";
import { fill, say } from "../../../core/lang";
import type { Lang } from "../../../core/lang";
import type { EndpointsAnswer } from "../../../wire";

export interface EndpointRowLook {
  readonly name: string;
  readonly label: string;
  readonly id: string | undefined;
  readonly local: string | undefined;
  readonly face: string;
  readonly baseUrl: string;
  readonly keyed: string;
  readonly models: readonly string[];
  readonly accounts: string;
}

export interface EndpointListLook {
  readonly rows: readonly EndpointRowLook[];
}

export function lookOf(answer: EndpointsAnswer, lang: Lang): EndpointListLook {
  return {
    rows: answer.endpoints.map((endpoint) => ({
      name: endpoint.name,
      label: endpoint.label,
      id: endpoint.label === endpoint.name ? undefined : endpoint.name,
      local: endpoint.local ? say(lang, "setup_local") : undefined,
      face: wireApiOf(endpoint.dialect),
      baseUrl: endpoint.base_url,
      keyed: say(lang, endpoint.has_credential ? "setup_keyed" : "setup_unkeyed"),
      models: endpoint.models.map((model) => model.id),
      accounts: fill(say(lang, "setup_accounts_toggle"), {
        n: String(endpoint.tuning.accounts?.length ?? 0),
      }),
    })),
  };
}
