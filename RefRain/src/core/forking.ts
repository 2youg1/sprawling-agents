// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The one request a verb makes of a screen rather than of the city.
// `/fork` with no address cannot name a line from inside the verb
// table - the conversation and its positions belong to the talk
// screen - so it asks for the picker instead, and that screen watches
// this count and opens it. Counted rather than flagged so a second ask
// while the picker is already open is heard rather than absorbed by a
// value that is already `true`.

import { readonly, writable } from "svelte/store";
import type { Readable } from "svelte/store";

const asked = writable(0);

export const forkAsked: Readable<number> = readonly(asked);

export function askFork(): void {
  asked.update((held) => held + 1);
}
