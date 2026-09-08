// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import table from "../lang.json";
import { endonym, fill, LANGS, langOf, say } from "./lang";

// The slot names a pattern uses, sorted, so two languages of one
// sentence can be compared for asking the same values.
function slotsOf(pattern: string): string[] {
  const found: string[] = [];
  for (const match of pattern.matchAll(/\{([^}]*)\}/g)) {
    const name = match[1];
    if (name !== undefined) {
      found.push(name);
    }
  }
  return found.sort();
}

describe("lang", () => {
  test("a phrase is said in the language asked for", () => {
    expect(say("en", "nav_cost")).toBe("cost");
    expect(say("zh", "nav_cost")).toBe("成本");
    expect(say("en", "dispatch_send")).toBe("send it");
    expect(say("zh", "dispatch_send")).toBe("派出去");
  });

  test("nothing is left untranslated or left as English by accident", () => {
    for (const [key, phrase] of Object.entries(table)) {
      expect(phrase.en.trim(), `${key} has no English`).not.toBe("");
      expect(phrase.zh.trim(), `${key} has no Chinese`).not.toBe("");
      expect(phrase.zh, `${key} was copied rather than translated`).not.toBe(
        phrase.en,
      );
    }
  });

  test("both languages of a sentence ask for the same values", () => {
    for (const [key, phrase] of Object.entries(table)) {
      expect(slotsOf(phrase.zh), `${key} fills different slots`).toEqual(
        slotsOf(phrase.en),
      );
    }
  });

  test("every key is the snake_case of a Msg variant", () => {
    for (const key of Object.keys(table)) {
      expect(key).toMatch(/^[a-z][a-z0-9_]*$/);
    }
  });

  test("a browser asking in Chinese is answered in Chinese", () => {
    for (const tag of ["zh", "zh-CN", "zh-Hans-CN", "ZH-TW"]) {
      expect(langOf(tag), tag).toBe("zh");
    }
    for (const tag of ["en", "en-GB", "de", "", "zzh"]) {
      expect(langOf(tag), tag).toBe("en");
    }
  });

  test("a slot nobody filled stays visible rather than disappearing", () => {
    expect(fill("{a} of {b}", { a: "3" })).toBe("3 of {b}");
    expect(fill("{a} of {b}", { a: "3", b: "4" })).toBe("3 of 4");
  });

  test("a language names itself in itself, and the switch offers both", () => {
    expect(endonym("zh")).toBe("中文");
    expect(endonym("en")).toBe("English");
    expect(LANGS).toEqual(["en", "zh"]);
  });
});
