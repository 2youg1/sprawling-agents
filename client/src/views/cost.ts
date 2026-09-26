// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What one `CostAnswer` lets a page say. `cost.svelte` draws the whole
// answer and `facts.svelte` draws two figures out of it, and both have
// to tell the same three situations apart: nothing has run, a provider
// priced what ran, and calls ran that no provider priced. The fold
// lives here so the two pages cannot answer that question differently.

import type { CostAnswer } from "../wire";

export type CostReading =
  | { readonly kind: "idle" }
  | { readonly kind: "unpriced"; readonly calls: number; readonly tokens: number }
  | { readonly kind: "priced" };

export function costReading(answer: CostAnswer): CostReading {
  return answer.total > 0 ? { kind: "priced" } : { kind: "idle" };
}
