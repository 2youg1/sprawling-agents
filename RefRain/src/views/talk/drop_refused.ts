// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Lang } from "../../core/lang";
import { fill, say } from "../../core/lang";
import type { Refused } from "./dropping";

export interface DropRefusedLook {
  readonly lines: readonly string[];
  readonly wire: { readonly role: "alert" };
}

export function lookOf(refused: readonly Refused[], lang: Lang): DropRefusedLook {
  return {
    lines: refused.map((each) =>
      fill(say(lang, "talk_drop_refused"), {
        name: each.name,
        why: each.said === "" ? say(lang, "talk_not_live") : each.said,
      }),
    ),
    wire: { role: "alert" },
  };
}
