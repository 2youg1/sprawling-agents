// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export type EarlierDrawn = "none" | "folded" | "open";

export function earlierDrawn(shown: number, earlier: number, branched: boolean): EarlierDrawn {
  if (earlier === 0) return "none";
  if (shown === 0) return branched ? "open" : "none";
  return "folded";
}
