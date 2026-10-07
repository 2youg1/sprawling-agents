// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the one back key of every page is handed (`back.look.svelte`):
// its words and where it leads. Where it leads is `parts/back.ts`'s;
// the key names the place it leads to.

import { say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import type { View } from "../../core/route";
import { toFragment } from "../../core/route";
import { backOf } from "../parts/back";

export interface BackLook {
  readonly label: string;
  // Spread on the link.
  readonly wire: { readonly href: string };
}

// `undefined` where the page draws no back key.
export function backLookOf(page: View, lang: Lang): BackLook | undefined {
  const back = backOf(page);
  if (back === null) return undefined;
  return {
    label: say(lang, back.kind === "city" ? "nav_city" : "nav_settings"),
    wire: { href: toFragment(back) },
  };
}
