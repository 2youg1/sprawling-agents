// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The two owners of an ordered account list the account editor serves:
// an attached provider endpoint, and one supplier of the city's web
// search. Each says where its keys are filed and which frame carries
// its whole list; the editor itself never learns which one it holds
// (client D93).

import { configureSearch, reattachEndpoint } from "../../../core/commands";
import { referenceFor, searchReferenceFor } from "../../../core/enrol";
import type { AccountRetries, AccountStatus, EndpointSummary, SearchConfiguration, SearchSupplier } from "../../../wire";
import type { Roster } from "./account_editor";

// The `Custom` arm of `[search]`: the selected supplier and the list.
export type CustomSearch = Extract<SearchConfiguration, { readonly custom: unknown }>["custom"];

// An attached endpoint. Its list goes back through `reattachEndpoint`
// with the tuning as the city read it out; the retry count it carries
// beside the list falls back to the city's figure, `fallback`, which
// the page reads rather than spells.
export function endpointRoster(endpoint: EndpointSummary, fallback: AccountRetries | undefined): Roster {
  const accounts = endpoint.tuning.accounts ?? null;
  return {
    kind: "provider",
    owner: endpoint.name,
    accounts,
    keys: endpoint.account_status,
    legacy: accounts === null && endpoint.has_credential,
    filed: (account) => referenceFor(endpoint.name, account),
    frame: (rows, retries) => reattachEndpoint(endpoint, rows, retries),
    retries: { chosen: endpoint.tuning.account_retries ?? null, fallback },
    stamp: JSON.stringify(endpoint),
  };
}

// One supplier of the city's `[search]`, inside the value the search
// card draws. Its list goes back as the whole `[search]` with only this
// supplier's accounts replaced, because the city writes the table whole
// (client D94). A search keeps no account affinity, and its retry
// count on one account is the city's default, with no setting.
export function supplierRoster(
  custom: CustomSearch,
  supplier: SearchSupplier,
  keys: readonly AccountStatus[],
  stamp: string,
): Roster {
  return {
    kind: "supplier",
    owner: supplier.id,
    accounts: supplier.accounts,
    keys,
    legacy: false,
    filed: (account) => searchReferenceFor(supplier.id, account),
    frame: (rows) =>
      configureSearch({
        custom: {
          selected: custom.selected,
          suppliers: custom.suppliers.map((each) => (each.id === supplier.id ? { ...each, accounts: [...rows] } : each)),
        },
      }),
    retries: undefined,
    stamp,
  };
}
