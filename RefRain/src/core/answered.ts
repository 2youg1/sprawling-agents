// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Answer } from "../wire";

export type Answered<T> =
  | { readonly kind: "asking" }
  | { readonly kind: "held"; readonly value: T }
  | { readonly kind: "unavailable"; readonly query: string; readonly reason: string | null };

export function readAnswer<T>(
  answer: Answer | undefined,
  pick: (answer: Answer) => T | undefined,
): Answered<T> {
  if (answer === undefined) return { kind: "asking" };
  if ("unavailable" in answer) {
    return {
      kind: "unavailable",
      query: answer.unavailable.query,
      reason: answer.unavailable.reason ?? null,
    };
  }
  const value = pick(answer);
  return value === undefined
    ? { kind: "unavailable", query: Object.keys(answer)[0] ?? "", reason: null }
    : { kind: "held", value };
}
