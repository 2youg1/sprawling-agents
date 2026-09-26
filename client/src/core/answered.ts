// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What one held answer is to the view that asked for it. A view asks one
// question and draws one variant; every other thing the slot can hold is
// still an answer, and a view that folds it into "still asking" or into
// "empty" tells a person the city is busy or bare when it could not look.

import type { Answer } from "../wire";

export type Answered<T> =
  | { readonly kind: "asking" }
  | { readonly kind: "held"; readonly value: T }
  // `query` is the question as the city spelled it back, so a person
  // reads which question went unanswered, not only that one did.
  | { readonly kind: "unavailable"; readonly query: string };

// `pick` reads the one variant the view draws and answers `undefined`
// for every other.
export function readAnswer<T>(answer: Answer | undefined, pick: (answer: Answer) => T | undefined): Answered<T> {
  if (answer === undefined) return { kind: "asking" };
  if ("unavailable" in answer) return { kind: "unavailable", query: answer.unavailable.query };
  const value = pick(answer);
  // A variant nobody asked this slot for is named by its own key: the
  // page then says what arrived instead of drawing an empty result.
  return value === undefined ? { kind: "unavailable", query: Object.keys(answer)[0] ?? "" } : { kind: "held", value };
}
