// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What one `CostAnswer` lets a page say. `cost.svelte` draws the whole
// answer and `facts.svelte` draws two figures out of it, and both have
// to tell the same three situations apart: nothing has run, a provider
// priced what ran, and calls ran that no provider priced. The fold
// lives here so the two pages cannot answer that question differently.

import type { Answered } from "../core/answered";
import type { CostAnswer, RunId } from "../wire";

export type CostReading =
  | { readonly kind: "idle" }
  | { readonly kind: "unpriced" }
  | { readonly kind: "priced" };

export function costReading(answer: CostAnswer): CostReading {
  if (answer.total > 0) return { kind: "priced" };
  return answer.unpriced.calls > 0 ? { kind: "unpriced" } : { kind: "idle" };
}

// What the facts row says one run has spent. A page with no run moving
// has nothing to say about a run, so that case answers `none` before the
// cost answer is consulted: a cost query the city could not answer is
// not a fact about a run that does not exist.
export type RunSpend =
  | { readonly kind: "none" }
  | { readonly kind: "unreadable" }
  | { readonly kind: "unpriced" }
  | { readonly kind: "spent"; readonly micros: number };

export function runSpend(read: Answered<CostAnswer>, run: RunId | null): RunSpend {
  if (read.kind === "unavailable") return { kind: "unreadable" };
  if (run === null || read.kind === "asking") return { kind: "none" };
  if (costReading(read.value).kind === "unpriced") return { kind: "unpriced" };
  const found = read.value.by_run.find(([name]) => name === run);
  return { kind: "spent", micros: found === undefined ? 0 : found[1] };
}
