// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the count of blocks that arrived while the person read back is
// given: the scroller (`scroller.svelte`) owns the count and the way
// back to the foot, and the look draws the one button that carries both.

export interface UnreadWire {
  readonly type: "button";
  // The count and what pressing it does, as one name.
  readonly "aria-label": string;
  readonly onclick: () => void;
}

export interface UnreadLook {
  // Already in the person's language.
  readonly text: string;
  readonly wire: UnreadWire;
}
