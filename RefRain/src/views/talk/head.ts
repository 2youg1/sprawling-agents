// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the head of one message gives its look (`head.look.svelte`):
// the speaker, the session facts where this head is the one that states
// them, the time, the two measured figures already in words, and the
// arrival rhythm where the page watched it.

import type { Snippet } from "svelte";

export interface HeadLook {
  readonly who: string;
  readonly model: string | null;
  // The time to the second, and the same instant for `<time datetime>`.
  readonly time: { readonly text: string; readonly iso: string };
  // Time to first content and the output rate, already in words, each
  // absent until the Ledger holds both its moments.
  readonly ttft: string | undefined;
  readonly tps: string | undefined;
  readonly rhythm: Snippet | undefined;
}
