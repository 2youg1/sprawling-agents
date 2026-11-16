// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How the palette's "speak" reaches the microphone of the composer on
// the page (client/Spec.lean §4-64b): the palette counts a request, and
// `record.svelte` presses its own button for each one made after it was
// drawn. A count rather than a flag, because asking again while it
// records is heard too: that is the second press, which stops it.

import { writable } from "svelte/store";
import type { Readable } from "svelte/store";

const asked = writable(0);

export const speakAsked: Readable<number> = asked;

export function askToSpeak(): void {
  asked.update((count) => count + 1);
}
