// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a rule across the conversation is given: one line of quiet words
// between two hairlines, which is how the thread marks every boundary a
// reader crosses - the earlier rounds folded away, where a branch came
// from, a new session, a run that ended. One look draws them all, so a
// boundary reads the same wherever it falls.

import type { FoldLook } from "./fold";

// One stretch of the line. Every word is already in the person's
// language.
export type RulePart =
  | { readonly kind: "text"; readonly text: string }
  | { readonly kind: "link"; readonly text: string; readonly href: string }
  | { readonly kind: "fold"; readonly fold: FoldLook };

// The bag spread on the rule itself: a run's ending speaks once, as a
// status, and names its phase for the read-wear bar (`data-wear`), which
// reads the attribute off the page; the other rules say neither.
export interface RuleWire {
  readonly role?: "status";
  readonly "data-wear"?: string;
}

export interface RuleLook {
  readonly parts: readonly RulePart[];
  // The branch mark before the words, on the rule that says where a
  // branch came from; no mark elsewhere.
  readonly mark: "branch" | "none";
  readonly wire: RuleWire;
}
