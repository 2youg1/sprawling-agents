// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Snippet } from "svelte";

export interface HeadLook {
  readonly who: string;
  readonly model: string | null;
  readonly time: { readonly text: string; readonly iso: string };
  readonly ttft: string | undefined;
  readonly tps: string | undefined;
  readonly rhythm: Snippet | undefined;
}
