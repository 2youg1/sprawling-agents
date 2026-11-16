// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export type TextKeyWire =
  | { readonly href: string }
  | { readonly type: "button"; readonly "aria-pressed"?: boolean; readonly onclick: () => void };

export interface TextKeyLook {
  readonly label: string;
  readonly wire: TextKeyWire;
}
