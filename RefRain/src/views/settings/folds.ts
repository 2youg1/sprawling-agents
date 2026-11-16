// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { SetupGroup, View } from "../../core/route";
import { nestOf, standingOf, standingForGroup, type Nest } from "./tree";

export interface Folds {
  readonly branch: number | null;
  readonly more: boolean;
  readonly nests: Readonly<Record<Nest, boolean>>;
}

export type Fold =
  | { readonly kind: "branch"; readonly at: number }
  | { readonly kind: "more" }
  | { readonly kind: "nest"; readonly nest: Nest };

function nestsOf(beneath: View): Folds["nests"] {
  const nest = nestOf(beneath);
  return { buildings: nest === "buildings", record: nest === "record" };
}

export function foldsOf(drawn: SetupGroup, beneath: View): Folds {
  const standing = standingOf(drawn, beneath);
  return {
    branch: standing.branch,
    more: standing.more,
    nests: nestsOf(beneath),
  };
}

export function foldsForGroup(drawn: SetupGroup, beneath: View): Folds {
  const standing = standingForGroup(drawn);
  return {
    branch: standing.branch,
    more: standing.more,
    nests: nestsOf(beneath),
  };
}

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
