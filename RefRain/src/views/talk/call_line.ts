// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { LineMove } from "../../core/lines";

export type CallFigure =
  | { readonly kind: "landed"; readonly text: string }
  | { readonly kind: "running"; readonly text: string }
  | { readonly kind: "none" };

export interface CallVerdict {
  readonly tone: "alert" | "quiet";
  readonly text: string;
}

export interface CallLineWire {
  readonly type: "button";
  readonly "data-call-line": "";
  readonly "aria-pressed": boolean;
  readonly onclick: () => void;
  readonly onkeydown: (event: KeyboardEvent) => void;
}

export interface CallLineLook {
  readonly kind: string;
  readonly subject: string;
  readonly running: boolean;
  readonly time: CallFigure;
  readonly verdict: CallVerdict | null;
  readonly failure: string | null;
  readonly moves: readonly LineMove[];
  readonly opened: boolean;
  readonly pinned: boolean;
  readonly hint: string;
  readonly wire: CallLineWire;
}
