// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The one way out of a page that has nothing to show because nobody
// has asked the city for anything yet: the conversation with the Mayor,
// where a person asks. The city page with no buildings, the cost page
// with nothing spent and the registry with nothing filed all offer it,
// so its words and its destination are decided here once.

import type { Lang } from "../../core/lang";
import { say } from "../../core/lang";
import { MAYOR, toFragment } from "../../core/route";

export interface AskMayorLook {
  readonly label: string;
  // Spread on the link.
  readonly wire: { readonly href: string };
}

export function lookOf(lang: Lang): AskMayorLook {
  return { label: say(lang, "city_ask_mayor"), wire: { href: toFragment({ kind: "talk", address: MAYOR }) } };
}
