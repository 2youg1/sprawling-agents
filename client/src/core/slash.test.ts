// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import table from "../lang.json";
import { SLASH, completed, find, offered, parse } from "./slash";

describe("slash", () => {
  test("every verb is spelled once, with a slash", () => {
    const seen = new Set<string>();
    for (const known of SLASH) {
      expect(known.spelling, "a verb begins with a slash").toMatch(/^\/[a-z]+$/);
      expect(seen.has(known.spelling), `${known.spelling} is spelled twice`).toBe(false);
      seen.add(known.spelling);
    }
  });

  // The defect §4.3 names: the pages label their buttons with command
  // spellings, and a person who typed one of those labels into the box
  // got nothing. A label and a verb are the same table or they are two,
  // and two is how they drifted apart.
  test("every command a button is labelled with is a verb somebody can type", () => {
    for (const [key, phrase] of Object.entries(table)) {
      if (!phrase.en.startsWith("/")) {
        continue;
      }
      const call = parse(phrase.en);
      expect(call, `${key} is not a command line`).not.toBeNull();
      expect(find(call?.verb ?? ""), `${key} spells a verb no table holds`).toBeDefined();
    }
  });

  test("a line is cut into the verb, its words, and the sentence after it", () => {
    expect(parse("hello")).toBeNull();
    expect(parse("/stop")).toEqual({ verb: "/stop", words: [], rest: "" });
    expect(parse("/stop --all")).toEqual({ verb: "/stop", words: ["--all"], rest: "--all" });
    expect(parse("/steer  read the city ")).toEqual({
      verb: "/steer",
      words: ["read", "the", "city"],
      rest: "read the city",
    });
  });

  test("the menu narrows as the verb is typed, then shows the one verb's grammar", () => {
    expect(offered("hello")).toHaveLength(0);
    expect(offered("/").length).toBe(SLASH.length);
    expect(offered("/st").map((each) => each.spelling)).toEqual(["/steer", "/stop"]);
    expect(offered("/stop ").map((each) => each.spelling)).toEqual(["/stop"]);
  });

  test("tab completes to the one match, or to the prefix every match shares", () => {
    expect(completed("/dis")).toBe("/dispatch ");
    // `/steer` and `/stop` share only the letter already typed, so the
    // line is returned untouched rather than silently shortened.
    expect(completed("/st")).toBe("/st");
    expect(completed("/m")).toBe("/m");
    expect(completed("/mc")).toBe("/mcp ");
    expect(completed("/zzz")).toBe("/zzz");
    expect(completed("read the city")).toBe("read the city");
  });
});
