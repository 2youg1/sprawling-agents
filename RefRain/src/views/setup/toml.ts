// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Everything the look of the folded `CONFIG.toml` is given; the seat is
// `toml.svelte` and the look `toml.look.svelte`. The platform's
// disclosure owns the keys and the open state, so the only bag is the
// region's name.

import type { Snippet } from "svelte";

export interface TomlLook {
  readonly region: { readonly "aria-label": string };
  // Already in the person's language.
  readonly toggle: string;
  // The file's path in the city, never translated.
  readonly path: string;
  // The file as the city holds it, or `undefined` while it is unread or
  // empty.
  readonly text: string | undefined;
  // What stands in the file's place while it is unread.
  readonly unread: Snippet;
}
