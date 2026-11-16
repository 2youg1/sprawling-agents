// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Snippet } from "svelte";
import type { HTMLDialogAttributes } from "svelte/elements";

export type Stands = "centre" | "below-top" | "in-flow";

export interface Cancelling {
  readonly preventDefault: () => void;
}

export type SheetWire = Omit<HTMLDialogAttributes, "oncancel"> & {
  readonly oncancel?: (event: Cancelling) => void;
};

export interface SheetLook {
  readonly wire: SheetWire;
  readonly stands: Stands;
  readonly children: Snippet;
}
