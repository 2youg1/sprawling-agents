// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The recycle bin as decided, with no look: which rows the page can
// bring back itself, what the press hands on, and the way back each
// row states in words.

import { beforeAll, describe, expect, test } from "bun:test";

import { Locator, TimeMs } from "../../wire";
import type { DiscardLine, Restoration } from "../../wire";
import { clock } from "../../core/time";
import { lookOf } from "./bin";

const AT = TimeMs.make(Date.UTC(2026, 9, 2, 12));

function line(restoration: Restoration, restored: boolean): DiscardLine {
  return { at: AT, path: "shop/notes/draft.md", restoration, restored };
}

const TRACKED: Restoration = { tracked: Locator.make("9c41e07:shop/notes/draft.md") };
const INTERRED: Restoration = { interred: Locator.make("interred/2026-10-02/draft.md") };
const REBUILT: Restoration = { rebuildable: { reason: "rebuilt on the next read" } };

describe("the recycle bin", () => {
  // Every row writes its moment through Intl, and the first locale-aware
  // date format in a Bun process loads the ICU data - about a second on
  // an idle machine, several on a loaded one. That cost is the runtime's,
  // so it is paid once here and no test's budget measures it.
  beforeAll(() => {
    clock("en", 0);
  });

  test("offers the press only on a tracked row that is still gone, and the press sends that row's way back", () => {
    const sent: Restoration[] = [];
    const rows = [line(TRACKED, false), line(TRACKED, true), line(INTERRED, false), line(REBUILT, false)];
    const look = lookOf(rows, "en", (restoration) => sent.push(restoration));
    expect(look.rows.map((row) => [row.standing, row.standingWord, row.restore?.label])).toEqual([
      ["gone", "gone", "put back"],
      ["restored", "restored", undefined],
      ["gone", "gone", undefined],
      ["gone", "gone", undefined],
    ]);
    look.rows[0]?.restore?.onPress();
    expect(sent).toEqual([TRACKED]);
  });

  test("every row states its own way back, one arm per storage authority", () => {
    const look = lookOf([line(TRACKED, false), line(INTERRED, false), line(REBUILT, false)], "en", () => undefined);
    expect(look.rows.map((row) => row.way)).toEqual([
      "in git at 9c41e07:shop/notes/draft.md",
      "in the store at interred/2026-10-02/draft.md",
      "rebuildable: rebuilt on the next read",
    ]);
  });

  test("a path discarded twice is two rows, told apart by the moment", () => {
    const later = { ...line(TRACKED, false), at: TimeMs.make(AT + 1) };
    const keys = lookOf([line(TRACKED, false), later], "en", () => undefined).rows.map((row) => row.key);
    expect(new Set(keys).size).toBe(2);
  });
});
