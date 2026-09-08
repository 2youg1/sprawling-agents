// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Every word a person reads comes from `lang.json`, in the two languages
// this client speaks. One entry holds both, so a translation cannot be
// missing: the type of the table is inferred from the file, and a key
// the file does not hold does not compile.

import table from "../lang.json";

export type Lang = "en" | "zh";

// Both, in the order a switch offers them.
export const LANGS: readonly Lang[] = ["en", "zh"];

export type Key = keyof typeof table;

// The word for one message in one language.
export function say(lang: Lang, key: Key): string {
  return table[key][lang];
}

// The language a browser asking for `tag` should be given. Prefix-matched,
// because a browser says `zh-CN` or `zh-Hans-CN` and means Chinese.
// Anything else is English, which is what this client is written in.
export function langOf(tag: string): Lang {
  return tag.toLowerCase().startsWith("zh") ? "zh" : "en";
}

// What a language calls itself. Never translated: a person looking for
// their own language looks for its own name.
export function endonym(lang: Lang): string {
  switch (lang) {
    case "en":
      return "English";
    case "zh":
      return "中文";
  }
}

// Fills the `{named}` slots of a phrase. Named rather than positional,
// because the two languages put them in different orders. A slot the
// caller did not fill is left as written, so it shows on screen: a
// visible `{runs}` is a defect somebody reports, and a silently emptied
// sentence is one nobody does.
export function fill(
  pattern: string,
  slots: Readonly<Record<string, string>>,
): string {
  let out = pattern;
  for (const [name, value] of Object.entries(slots)) {
    out = out.replaceAll(`{${name}}`, value);
  }
  return out;
}
