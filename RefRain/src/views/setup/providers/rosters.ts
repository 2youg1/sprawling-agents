// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { configureSearch, reattachEndpoint } from "../../../core/commands";
import { referenceFor, searchReferenceFor } from "../../../core/enrol";
import type {
  AccountRetries,
  AccountStatus,
  EndpointSummary,
  SearchConfiguration,
  SearchSupplier,
} from "../../../wire";
import type { Roster } from "./account_editor";

export type CustomSearch = Extract<SearchConfiguration, { readonly custom: unknown }>["custom"];

export function endpointRoster(
  endpoint: EndpointSummary,
  fallback: AccountRetries | undefined,
): Roster {
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
          suppliers: custom.suppliers.map((each) =>
            each.id === supplier.id ? { ...each, accounts: [...rows] } : each,
          ),
        },
      }),
    retries: undefined,
    stamp,
  };
}
