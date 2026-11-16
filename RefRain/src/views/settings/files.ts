// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Where the files the settings panel reads back live in the city's tree,
// as addresses `Query::Document` takes. The city decides these places
// (`kernel::layout`, `city::policy::rules_path`), and the wire carries
// no path for them, so this is the one page-side spelling of each; the
// governed documents keep theirs beside their editor
// (`views/setup/governed.svelte`) for the same reason.

import { Address } from "../../wire";

// The city's own layer: `<city>/.sprawling/CONFIG.toml`.
export const CITY_CONFIG: Address = Address.make(".sprawling/CONFIG.toml");

// The building's rules: `<building>/.sprawling/RULES.toml`.
export function rulesAt(building: Address): Address {
  return Address.make(`${building}/.sprawling/RULES.toml`);
}
