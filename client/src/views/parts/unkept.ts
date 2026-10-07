// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a look of the unkept-draft line is given (`unkept.svelte`).

import type { CopyProps } from "./copy";

export interface UnkeptLook {
  // The one sentence, in the reader's language.
  readonly sentence: string;
  // Spread on the line: a polite live region, so the sentence is heard
  // when the browser first refuses the draft.
  readonly line: { readonly role: "status" };
  readonly copy: CopyProps;
}
