// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The file finder as an APG Combobox (client/Spec.lean §4-62): the keys
// in the box, the active option it names, and the sentences that say
// what the city could not, read from the value every look is handed.

import { describe, expect, test } from "bun:test";

import { fill, say } from "../../core/lang";
import { Address } from "../../wire";
import type { FindAnswer } from "../../wire";
import type { SearchHands, SearchState } from "./search";
import { optionId, partsOf, searchLookOf } from "./search";

const LAB = Address.make("lab");

function state(found: FindAnswer | undefined, cursor = 0): SearchState {
  return { under: LAB, titleId: "t", uid: "f", text: "ma", cursor, found, lang: "en" };
}

function record(): { readonly typed: string[]; readonly pointed: number[]; readonly opened: string[]; readonly hands: SearchHands } {
  const typed: string[] = [];
  const pointed: number[] = [];
  const opened: string[] = [];
  return {
    typed,
    pointed,
    opened,
    hands: {
      type: (text) => typed.push(text),
      point: (at) => pointed.push(at),
      open: (path) => opened.push(path),
    },
  };
}

const FOUND: FindAnswer = {
  under: LAB,
  text: "ma",
  paths: ["src/main.rs", "Makefile"].map((path) => Address.make(path)),
  walked: "whole",
};

describe("the file finder", () => {
  test("a path is drawn as its name, then its directory", () => {
    expect([partsOf("src/views/main.ts"), partsOf("Makefile")]).toEqual([
      { name: "main.ts", dir: "src/views" },
      { name: "Makefile", dir: "" },
    ]);
  });

  test("the box names the active option and the list it controls", () => {
    const look = searchLookOf(state(FOUND, 1), record().hands);
    expect([look.box["aria-activedescendant"], look.box["aria-controls"], look.box["aria-expanded"]]).toEqual([
      optionId("f", 1),
      look.list.id,
      true,
    ]);
    expect(look.options.map((option) => [option.wire.id, option.wire["aria-selected"], option.active])).toEqual([
      ["f-0", false, false],
      ["f-1", true, true],
    ]);
  });

  test("↓ stops at the last path, Enter opens the active one, a letter is the box's own", () => {
    const seen = record();
    const at = (cursor: number) => searchLookOf(state(FOUND, cursor), seen.hands).box;
    const prevented: string[] = [];
    const press = (key: string) => ({
      key,
      preventDefault: () => {
        prevented.push(key);
      },
    });
    at(1).onkeydown(press("ArrowDown"));
    at(0).onkeydown(press("ArrowUp"));
    at(1).onkeydown(press("Enter"));
    at(1).onkeydown(press("m"));
    expect([seen.pointed, seen.opened, prevented]).toEqual([[1, 0], ["Makefile"], ["ArrowDown", "ArrowUp", "Enter"]]);
  });

  test("with no paths the box names no option, and Enter opens nothing", () => {
    const seen = record();
    const look = searchLookOf(state({ under: LAB, text: "zz", paths: [], walked: "cut" }), seen.hands);
    look.box.onkeydown({ key: "Enter", preventDefault: () => undefined });
    expect([look.box["aria-activedescendant"], look.box["aria-expanded"], seen.opened]).toEqual([undefined, false, []]);
    expect(look.notes).toEqual([fill(say("en", "finder_none"), { text: "zz" }), say("en", "finder_cut")]);
  });

  test("the pointer moves the cursor and a click opens the path under it", () => {
    const seen = record();
    const look = searchLookOf(state(FOUND), seen.hands);
    look.options[1]?.wire.onmouseenter();
    look.options[1]?.wire.onclick();
    look.box.oninput({ currentTarget: { value: "mak" } });
    expect([seen.pointed, seen.opened, seen.typed]).toEqual([[1], ["Makefile"], ["mak"]]);
  });
});
