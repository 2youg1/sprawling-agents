// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a box of several lines on the building page is given (client D95:
// the page's sections are the seats, `text_box.look.svelte` draws the
// box): a spine document being edited, or one of the sandbox's lists
// written one entry per line.

// How much room the box opens with: a whole document, or a short list.
export type TextBoxSize = "document" | "list";

export interface TextBoxWire {
  readonly value: string;
  readonly "aria-label": string | undefined;
  readonly "aria-invalid": boolean;
  readonly oninput: (event: Event & { readonly currentTarget: HTMLTextAreaElement }) => void;
}

export interface TextBoxLook {
  readonly size: TextBoxSize;
  readonly wire: TextBoxWire;
}

// The bag for a box holding `value`; `label` names it when no `<label>`
// around it does, and `edit` receives every change.
export function textBoxWire(
  value: string,
  label: string | undefined,
  invalid: boolean,
  edit: (value: string) => void,
): TextBoxWire {
  return {
    value,
    "aria-label": label,
    "aria-invalid": invalid,
    oninput: (event) => {
      edit(event.currentTarget.value);
    },
  };
}
