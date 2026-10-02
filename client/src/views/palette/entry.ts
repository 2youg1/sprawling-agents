// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One row of the palette, the rows grouped by the section a verb names,
// and which of the two lists is showing: the shapes the box builds and
// `rows.svelte` draws.

import type { Action } from "../../core/keys";
import type { Key } from "../../core/lang";
import type { Section } from "../../core/slash_hands";

export interface Entry {
  readonly label: string;
  readonly hint: string;
  // Present on a page a key reaches: the row shows that key's chord
  // where other rows show their hint.
  readonly action?: Action | undefined;
  readonly act: () => void;
  // Present means the verb cannot run here, and names why.
  readonly why?: Key | undefined;
}

export interface Group {
  readonly section: Section;
  readonly entries: readonly Entry[];
}

// Places to go, or - once the line begins with `/` - the verbs.
export type Listing = { readonly kind: "verbs"; readonly groups: readonly Group[] } | { readonly kind: "places" };
