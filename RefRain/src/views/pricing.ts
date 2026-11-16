// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Answered } from "../core/answered";
import type { CostAnswer, Renewals, RunId } from "../wire";

export type CostReading =
  { readonly kind: "idle" } | { readonly kind: "unpriced" } | { readonly kind: "priced" };

export function costReading(answer: CostAnswer): CostReading {
  if (answer.total > 0) return { kind: "priced" };
  return answer.unpriced.calls > 0 ? { kind: "unpriced" } : { kind: "idle" };
}

export type RunSpend =
  | { readonly kind: "none" }
  | { readonly kind: "unreadable" }
  | { readonly kind: "unpriced" }
  | { readonly kind: "spent"; readonly micros: number };

export function runSpend(read: Answered<CostAnswer>, run: RunId | null): RunSpend {
  if (run === null) return { kind: "none" };
  if (read.kind === "unavailable") return { kind: "unreadable" };
  if (read.kind === "asking") return { kind: "none" };
  if (costReading(read.value).kind === "unpriced") return { kind: "unpriced" };
  const found = read.value.by_run.find(([name]) => name === run);
  return { kind: "spent", micros: found === undefined ? 0 : found[1] };
}

export function renewalsSent(renewals: Renewals): number {
  return renewals.read + renewals.late + renewals.unreported + renewals.refused;
}
