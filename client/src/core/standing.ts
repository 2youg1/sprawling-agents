// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Where one asked question stands: still being asked, answered, or
// answered with "the city cannot answer this". A screen that narrows
// the held answer to the one arm it draws reads the third case as the
// first, and then shows "…" for ever over a checkpoint the city cannot
// read or a building that does not exist.

import type { Answer } from "../wire";

export type Standing<T> =
  | { readonly kind: "asking" }
  | { readonly kind: "unavailable" }
  | { readonly kind: "answered"; readonly value: T };

export function standing<T>(
  held: Answer | undefined,
  pick: (answer: Answer) => T | undefined,
): Standing<T> {
  const value = held === undefined ? undefined : pick(held);
  return value === undefined ? { kind: "asking" } : { kind: "answered", value };
}
