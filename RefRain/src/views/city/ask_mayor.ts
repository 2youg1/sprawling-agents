// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Lang } from "../../core/lang";
import { say } from "../../core/lang";
import { MAYOR, toFragment } from "../../core/route";

export interface AskMayorLook {
  readonly label: string;
  readonly wire: { readonly href: string };
}

export function lookOf(lang: Lang): AskMayorLook {
  return {
    label: say(lang, "city_ask_mayor"),
    wire: { href: toFragment({ kind: "talk", address: MAYOR }) },
  };
}
