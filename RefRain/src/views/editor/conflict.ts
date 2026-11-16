// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the conflict bar's look is given (client D95 shape: a seat, a
// look, and this file).

import type { Snippet } from "svelte";

// The bag spread on the bar: it is announced once, as an alert, when the
// draft is found to stand on a version the city no longer holds.
export interface ConflictWire {
  readonly role: "alert";
}

export interface ConflictLook {
  // The sentence that says what happened, already in the person's
  // language.
  readonly text: string;
  readonly wire: ConflictWire;
  // Compare, carry over and drop, drawn by their own parts.
  readonly actions: Snippet;
}
