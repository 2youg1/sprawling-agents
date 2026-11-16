// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { FoldLook } from "./fold";

export type RulePart =
  | { readonly kind: "text"; readonly text: string }
  | { readonly kind: "link"; readonly text: string; readonly href: string }
  | { readonly kind: "fold"; readonly fold: FoldLook };

export interface RuleWire {
  readonly role?: "status";
  readonly "data-wear"?: string;
}

export interface RuleLook {
  readonly parts: readonly RulePart[];
  readonly mark: "branch" | "none";
  readonly wire: RuleWire;
}
