// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export interface FoldWire {
  readonly type: "button";
  readonly "aria-expanded": boolean;
  readonly onclick: () => void;
}

export interface FoldLook {
  readonly label: string;
  readonly wire: FoldWire;
}

export function foldWire(open: boolean, toggle: () => void): FoldWire {
  return { type: "button", "aria-expanded": open, onclick: toggle };
}
