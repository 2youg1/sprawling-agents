// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import table from "../lang.json";

export type Lang = "en" | "zh";

export const LANGS: readonly Lang[] = ["en", "zh"];

export type Key = keyof typeof table;

export function isKey(text: string): text is Key {
  return Object.hasOwn(table, text);
}

export function say(lang: Lang, key: Key): string {
  return table[key][lang];
}

export function langOf(tag: string): Lang {
  return tag.toLowerCase().startsWith("zh") ? "zh" : "en";
}

export function endonym(lang: Lang): string {
  switch (lang) {
    case "en":
      return "English";
    case "zh":
      return "中文";
  }
}

export function fill(pattern: string, slots: Readonly<Record<string, string>>): string {
  let out = pattern;
  for (const [name, value] of Object.entries(slots)) {
    out = out.replaceAll(`{${name}}`, value);
  }
  return out;
}
