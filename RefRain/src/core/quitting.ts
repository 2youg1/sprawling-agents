// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// `/quit` asks a question before it closes the city, and the verb table
// cannot draw one, so it asks the shell instead: the shell watches this
// count and opens the question (`views/quit.svelte`). Counted rather
// than flagged, as `forking.ts` is, so a second ask while the question
// stands is heard.

import { readonly, writable } from "svelte/store";
import type { Readable } from "svelte/store";

const asked = writable(0);

export const quitAsked: Readable<number> = readonly(asked);

export function askQuit(): void {
  asked.update((held) => held + 1);
}
