// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What is attached, one row each, as the whole value a look draws
// (`EndpointListLook`): the list that stands over the form a key is
// entered through. Nothing here touches the DOM or uses a rune.
//
// **The row is headed by the name the person gave it.** `label` is the
// display name, and the city answers with the id when nobody stated
// one, so this row never has to decide what to show. The id appears
// beside it only when the two differ, because that is when a person
// needs both: the label to recognise the provider, and the id to find
// its table in a `config.toml` and its key in the vault.
//
// **The badge says the call never leaves this machine.** The city
// decides that from the base URL and states it as `local`, and a
// confidential building is refused every endpoint without it
// (`gateway::router::book::select`), so it is the one property of a row
// that changes what a person may do with it.

import { wireApiOf } from "../../../core/commands";
import { fill, say } from "../../../core/lang";
import type { Lang } from "../../../core/lang";
import type { EndpointsAnswer } from "../../../wire";

export interface EndpointRowLook {
  // The id commands name the endpoint by; also the key the row's
  // account editor is drawn under.
  readonly name: string;
  readonly label: string;
  // The id, present only when it differs from the label.
  readonly id: string | undefined;
  // The badge's word, present only for an endpoint whose calls never
  // leave this machine.
  readonly local: string | undefined;
  // The wire value `chat` / `responses` / `messages`, spelled the same
  // in both languages.
  readonly face: string;
  readonly baseUrl: string;
  readonly keyed: string;
  readonly models: readonly string[];
  // The disclosure the account editor folds under, named with how many
  // accounts it lists, because the list is read far more often than it
  // is reordered.
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
      accounts: fill(say(lang, "setup_accounts_toggle"), { n: String(endpoint.tuning.accounts?.length ?? 0) }),
    })),
  };
}
