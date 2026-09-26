// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Hunk } from "./trace";

export interface Reverse {
  readonly now: string;
  readonly was: string;
}

export function reverseOf(_hunk: Hunk): Reverse {
  return { now: "", was: "" };
}

export function lineOf(_hunk: Hunk): number {
  return 1;
}
