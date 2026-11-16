// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Key } from "../core/lang";
import type { PairWhy } from "../core/local/entering";

export const WHY: Readonly<Record<PairWhy, Key | undefined>> = {
  first: undefined,
  open_used: "pair_open_used",
  session_lost: "pair_session_lost",
  keyless: "pair_keyless",
};

export function codeOf(typed: string): string {
  return typed.replace(/\s/g, "").toLowerCase();
}
