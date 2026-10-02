// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The heading of one notice (client/Spec.lean §4-35), and the next step under
// it: the sentences this client can say in the reader's own language,
// found under `err_` and `recover_` plus the code in lower case.
//
// The heading is read under `err_`. A code whose key `lang.json` does not hold
// has no translation yet, and the title then shows that key name
// itself: a visible `err_e_foo` is a defect somebody reports, and a
// silently chosen default sentence is one nobody does (`fill` leaves
// its unfilled slots visible for the same reason).

import { askedIn } from "../../core/asking";
import { fill, say } from "../../core/lang";
import type { Key, Lang } from "../../core/lang";
import table from "../../lang.json";

function isKey(raw: string): raw is Key {
  return Object.hasOwn(table, raw);
}

//
// One heading is more specific than its code: a question this page
// asked and never heard back on names that question, because a page
// asks several at once and a person reading the same words twice cannot
// tell which of them the city left unanswered.
export function noticeTitle(lang: Lang, code: string, subject: string): string {
  const asked = code === "E_TIMEOUT" ? askedIn(subject) : null;
  if (asked !== null) return fill(say(lang, "ask_late_title"), { query: asked });
  const key = `err_${code.toLowerCase()}`;
  return isKey(key) ? say(lang, key) : key;
}

// What a person can do next, by the code, in the reader's language. The
// city writes its recovery in English and for the one case in front of
// it; the page shows its own sentence for every code it knows, and the
// city's only for a code this build has no word for, since a sentence in
// the wrong language still beats none. The city's own sentence stays
// reachable in the notice's detail.
export function recoveryWords(lang: Lang, code: string, said: string): string {
  const key = `recover_${code.toLowerCase()}`;
  return isKey(key) ? say(lang, key) : said;
}
