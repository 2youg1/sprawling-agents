// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What one line under RefRain's head is given (client D95 shape: the
// look and this file; the line has no wiring, so it has no seat).

import type { Snippet } from "svelte";

export interface LineLook {
  // `faint` explains; `alert` names something the person has to act on.
  readonly tone: "faint" | "alert";
  // The words, already in the person's language, or a part that says them.
  readonly children: Snippet;
}
