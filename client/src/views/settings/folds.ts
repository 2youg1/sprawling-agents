// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The settings tree's folds: which branch is open, whether its "more"
// is, and which nests are. The properties these moves keep are
// `client/spec/Views/Fold.lean`; `tree.test.ts` holds this file to them.

import type { SetupGroup, View } from "../../core/route";
import { nestOf, standingOf, type Nest } from "./tree";
// Which folds of the tree are open: the one branch, that branch's
// "more", and each nest. One branch at a time (client D53,
// `client/spec/Views/Fold.lean`).
export interface Folds {
  readonly branch: number | null;
  readonly more: boolean;
  readonly nests: Readonly<Record<Nest, boolean>>;
}

// A press on one of the buttons that open a list of their own.
export type Fold =
  | { readonly kind: "branch"; readonly at: number }
  | { readonly kind: "more" }
  | { readonly kind: "nest"; readonly nest: Nest };

// The folds the tree opens with: the branch the panel stands in, its
// "more" when the standing entry is inside it, and the nest of the page
// beneath.
export function foldsOf(drawn: SetupGroup, beneath: View): Folds {
  const standing = standingOf(drawn, beneath);
  const nest = nestOf(beneath);
  return { branch: standing.branch, more: standing.more, nests: { buildings: nest === "buildings", record: nest === "record" } };
}

// The folds after a press. Opening another branch folds the one that
// was open, and its "more" with it; pressing the open branch folds it.
export function toggled(folds: Folds, fold: Fold): Folds {
  switch (fold.kind) {
    case "branch":
      return { ...folds, branch: folds.branch === fold.at ? null : fold.at, more: false };
    case "more":
      return { ...folds, more: !folds.more };
    case "nest":
      return { ...folds, nests: { ...folds.nests, [fold.nest]: !folds.nests[fold.nest] } };
  }
}
