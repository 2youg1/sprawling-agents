// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the look under the box is handed for the files a drop did not
// keep: one line per file in the person's language, and the wiring that
// makes each line an alert, so a screen reader hears a file go missing
// as a sighted reader sees it (`drop_zone.svelte.ts`).

import type { Lang } from "../../core/lang";
import { fill, say } from "../../core/lang";
import type { Refused } from "./dropping";

export interface DropRefusedLook {
  readonly lines: readonly string[];
  // Spread on each line.
  readonly wire: { readonly role: "alert" };
}

// An empty reason is a city the page did not reach (`dropping.ts`
// `keep`), worded as such rather than as a blank after the colon.
export function lookOf(refused: readonly Refused[], lang: Lang): DropRefusedLook {
  return {
    lines: refused.map((each) =>
      fill(say(lang, "talk_drop_refused"), { name: each.name, why: each.said === "" ? say(lang, "talk_not_live") : each.said }),
    ),
    wire: { role: "alert" },
  };
}
