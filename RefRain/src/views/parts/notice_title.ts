// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { askedIn } from "../../core/asking";
import { fill, isKey, say } from "../../core/lang";
import type { Lang } from "../../core/lang";

export function noticeTitle(lang: Lang, code: string, subject: string): string {
  const asked = code === "E_TIMEOUT" ? askedIn(subject) : null;
  if (asked !== null) return fill(say(lang, "ask_late_title"), { query: asked });
  const key = `err_${code.toLowerCase()}`;
  return isKey(key) ? say(lang, key) : key;
}

export function recoveryWords(lang: Lang, code: string, said: string): string {
  const key = `recover_${code.toLowerCase()}`;
  return isKey(key) ? say(lang, key) : said;
}
