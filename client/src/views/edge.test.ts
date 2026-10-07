// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The wiring the edge key look is handed (docs/frontend-method.md §7E):
// the layers key's name says the tier, its ticks hold exactly that
// tier, its four handlers are the seat's, and the settings key is a
// link, so a middle click opens a tab.

import { describe, expect, test } from "bun:test";

import { TIERS } from "../core/prefs";
import { layersKey, settingsKey } from "./edge";
import type { LayersHands } from "./edge";

const HANDS: LayersHands = {
  press: () => undefined,
  release: () => undefined,
  click: () => undefined,
};

const chord = (name: string): string => `${name} [chord]`;

describe("the layers key", () => {
  test("names the tier it is in and holds that tick alone", () => {
    for (const tier of TIERS) {
      const look = layersKey(tier, "en", chord, HANDS);
      const name = look.key.as === "button" ? look.key.wire["aria-label"] : undefined;
      expect(name).toBe(`layers · ${tier}`);
      expect(look.hint).toBe(`layers · ${tier} [chord]`);
      expect(look.foot).toEqual({ kind: "tiers", ticks: TIERS.map((each) => ({ key: each, held: each === tier })) });
    }
  });

  test("hands its press, release and click to the seat", () => {
    const look = layersKey("zen", "en", chord, HANDS);
    expect(look.key).toEqual({
      as: "button",
      wire: {
        type: "button",
        "aria-label": "layers · zen",
        onpointerdown: HANDS.press,
        onpointerup: HANDS.release,
        onpointerleave: HANDS.release,
        onclick: HANDS.click,
      },
    });
  });
});

describe("the settings key", () => {
  test("is a link to the settings sheet, with nothing marked on it", () => {
    expect(settingsKey("en", chord, "#/setup")).toEqual({
      glyph: "settings",
      hint: "settings [chord]",
      foot: { kind: "none" },
      corner: { kind: "none" },
      key: { as: "link", wire: { href: "#/setup", "aria-label": "settings" } },
    });
  });
});
