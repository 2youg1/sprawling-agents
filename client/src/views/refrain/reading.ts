// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { say } from "../../core/lang";
import type { Key, Lang } from "../../core/lang";
import type { B3Hash } from "../../wire";

// The four ways RefRain reads a document (client/Spec.lean §7N).
export type Reading = "source" | "preview" | "diff" | "versions";

// A version as a person names it on the screen: its first seven hex
// digits, the way a commit is named.
export function short(version: B3Hash): string {
  return version.slice(0, 7);
}

// CodeMirror's own words, in the page's language (client D23):
// the editor and every comparison drawn with it read this one table.
const PHRASES: Readonly<Record<string, Key>> = {
  Find: "refrain_cm_find",
  Replace: "refrain_cm_replace",
  next: "refrain_cm_next",
  previous: "refrain_cm_previous",
  all: "refrain_cm_all",
  "match case": "refrain_cm_match_case",
  regexp: "refrain_cm_regexp",
  "by word": "refrain_cm_by_word",
  replace: "refrain_cm_replace_one",
  "replace all": "refrain_cm_replace_all",
  close: "refrain_cm_close",
  "current match": "refrain_cm_current",
  "on line": "refrain_cm_on_line",
  "replaced $ matches": "refrain_cm_replaced",
  "replaced match on line $": "refrain_cm_replaced_line",
  "Control character": "refrain_cm_control",
};

export function phrasesIn(lang: Lang): Readonly<Record<string, string>> {
  return Object.fromEntries(Object.entries(PHRASES).map(([phrase, key]) => [phrase, say(lang, key)]));
}
