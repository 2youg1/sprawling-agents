// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export interface DraftWire {
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
