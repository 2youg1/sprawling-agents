// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The heading of one notice (client-SPEC 4-35): the sentence this
// client can say in the reader's own language, found under `err_` plus
// the code in lower case. A code whose key `lang.json` does not hold
// has no translation yet, and the title then shows that key name
// itself: a visible `err_e_foo` is a defect somebody reports, and a
// silently chosen default sentence is one nobody does (`fill` leaves
// its unfilled slots visible for the same reason).

import { say } from "../../core/lang";
import type { Key, Lang } from "../../core/lang";
import table from "../../lang.json";

function isKey(raw: string): raw is Key {
  return Object.hasOwn(table, raw);
}

export function noticeTitle(lang: Lang, code: string, subject: string): string {
  const key = `err_${code.toLowerCase()}`;
  return isKey(key) ? say(lang, key) : key;
}
