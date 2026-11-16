// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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
