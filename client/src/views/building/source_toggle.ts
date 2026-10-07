// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the key that shows a Markdown file's source is given (client D95:
// `file.svelte` is the seat, `source_toggle.look.svelte` draws the key).
// It is a toggle, so the bag says whether it is pressed; the paint alone
// told only a sighted reader that the source was showing.

export interface SourceToggleWire {
  readonly type: "button";
  readonly "aria-pressed": boolean;
  readonly onclick: () => void;
}

export interface SourceToggleLook {
  // The name of the format whose source the key shows.
  readonly label: string;
  readonly pressed: boolean;
  readonly wire: SourceToggleWire;
}

export function sourceToggleWire(pressed: boolean, flip: () => void): SourceToggleWire {
  return { type: "button", "aria-pressed": pressed, onclick: flip };
}
