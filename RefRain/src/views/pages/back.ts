// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import type { View } from "../../core/route";
import { toFragment } from "../../core/route";
import { backOf } from "../parts/back";

export interface BackLook {
  readonly label: string;
  readonly wire: { readonly href: string };
}

export function backLookOf(page: View, lang: Lang): BackLook | undefined {
  const back = backOf(page);
  if (back === null) return undefined;
  return {
    label: say(lang, "nav_city"),
    wire: { href: toFragment(back) },
  };
}
