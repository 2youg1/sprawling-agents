// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Action } from "../../core/keys";
import type { Key } from "../../core/lang";
import type { Section } from "../../core/slash_hands";
import type { View } from "../../core/route";

export interface Entry {
  readonly label: string;
  readonly hint: string;
  readonly keyAction?: Action | undefined;
  readonly act: () => void;
  readonly to?: View | undefined;
  readonly why?: Key | undefined;
  readonly keeps?: true | undefined;
}

export interface Group {
  readonly section: Section;
  readonly entries: readonly Entry[];
}

export type Listing =
  { readonly kind: "verbs"; readonly groups: readonly Group[] } | { readonly kind: "places" };
