// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { readonly, writable } from "svelte/store";
import type { Readable } from "svelte/store";

const asked = writable(0);

export const quitAsked: Readable<number> = readonly(asked);

export function askQuit(): void {
  asked.update((held) => held + 1);
}
