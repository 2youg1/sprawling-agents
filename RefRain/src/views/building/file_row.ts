// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export interface FileRowWire {
  readonly type: "button";
  readonly "aria-label": string;
  readonly "aria-current": "true" | undefined;
  readonly onclick: () => void;
}

export interface FileRowLook {
  readonly how: string;
  readonly path: string;
  readonly lines: string;
  readonly current: boolean;
  readonly wire: FileRowWire;
}

export function fileRowWire(label: string, current: boolean, open: () => void): FileRowWire {
  return {
    type: "button",
    "aria-label": label,
    "aria-current": current ? "true" : undefined,
    onclick: open,
  };
}
