// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The text box a governed document is edited in. The bag the look
// spreads on the `<textarea>` is built here; the seat is
// `governed.svelte`, which owns the draft, and the look is
// `draft.look.svelte`.

export interface DraftWire {
  // Already in the person's language: the document being edited.
  readonly "aria-label": string;
  readonly value: string;
  readonly oninput: (event: { readonly currentTarget: { readonly value: string } }) => void;
}

export interface DraftLook {
  readonly wire: DraftWire;
}

export function draftOf(label: string, value: string, onEdit: (text: string) => void): DraftLook {
  return {
    wire: {
      "aria-label": label,
      value,
      oninput: (event) => {
        onEdit(event.currentTarget.value);
      },
    },
  };
}
