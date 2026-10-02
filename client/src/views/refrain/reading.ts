// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { B3Hash } from "../../wire";

// The four ways RefRain reads a document (client-SPEC 7N).
export type Reading = "source" | "preview" | "diff" | "versions";

// A version as a person names it on the screen: its first seven hex
// digits, the way a commit is named.
export function short(version: B3Hash): string {
  return version.slice(0, 7);
}
