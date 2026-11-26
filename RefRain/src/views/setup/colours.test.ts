// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { get } from "../../core/store";

import { apcaLc, apcaOn, type Rgba } from "../../core/apca";
import vectors from "../../theme/apca-vectors.json";
import { loadPreferences } from "../../core/prefs";
import { BUILT_IN_THEME } from "../../core/theme_override";
import { memory } from "../../core/rows";
import {
  colourTokens,
  readLegibility,
  textClaims,
  wearKeptTheme,
  wearTheme,
  type Inline,
  type Stage,
} from "./colours";

function inline(start: Record<string, string>): Inline & { readonly held: Map<string, string> } {
  const held = new Map(Object.entries(start));
  return {
    held,
    get length() {
      return held.size;
    },
    item: (index) => [...held.keys()][index] ?? "",
    setProperty: (name, value) => {
      held.set(name, value);
    },
    removeProperty: (name) => {
      const was = held.get(name) ?? "";
      held.delete(name);
      return was;
    },
  };
}

function stage(
  start: Record<string, string>,
): Stage & { readonly style: ReturnType<typeof inline> } {
  return { style: inline(start), sheet: { textContent: "" } };
}

describe("the colour tokens offered", () => {
  test("are the declared --color- variables, once each, in the order the stylesheet declares them", () => {
    expect(
      colourTokens([
        "--color-g0",
        "--chroma",
        "--color-text",
        "--color-g0",
        "--tier-text",
        "--color-accent",
      ]),
    ).toEqual(["--color-g0", "--color-text", "--color-accent"]);
  });
});

describe("wearing an override", () => {
  test("lays each token on the root and the text in the one style element, and restoring default gives back the built-in page value by value", () => {
    const page = stage({ "--body-size": "16px", "--blend": "40%" });
    const before = new Map(page.style.held);
    wearTheme(page, {
      tokens: { "--color-g2": "#202020", "--color-accent": "oklch(0.7 0.1 30)" },
      css: ".x { color: red }",
    });
    expect(page.style.held.get("--color-g2")).toBe("#202020");
    expect(page.sheet.textContent).toBe(".x { color: red }");

    wearTheme(page, { tokens: { "--color-g2": "#303030" }, css: null });
    expect(page.style.held.has("--color-accent")).toBe(false);

    wearTheme(page, BUILT_IN_THEME);
    expect(page.style.held).toEqual(before);
    expect(page.sheet.textContent).toBe("");
  });

  test("a changed token survives a reload and reaches a second browser through the city's answer", () => {
    const rows = memory();
    const door = loadPreferences(rows, "en");
    door.setTheme({ tokens: { "--color-accent": "#3366ff" }, css: null });

    const reloaded = stage({});
    wearKeptTheme(reloaded, loadPreferences(rows, "en"));
    expect(reloaded.style.held.get("--color-accent")).toBe("#3366ff");

    const elsewhere = loadPreferences(memory(), "en");
    const second = stage({});
    wearKeptTheme(second, elsewhere);
    elsewhere.adopt(
      { ...get(elsewhere.held), theme: { tokens: { "--color-accent": "#3366ff" }, css: null } },
      [],
    );
    expect(second.style.held.get("--color-accent")).toBe("#3366ff");
  });
});

describe("legibility", () => {
  test("APCA matches shared signed sRGB reference observations", () => {
    for (const { name, text, surface, backdrop, lc } of vectors) {
      const [tr, tg, tb, ta] = text;
      const [sr, sg, sb, sa] = surface;
      const [br, bg, bb] = backdrop;
      if (
        tr === undefined ||
        tg === undefined ||
        tb === undefined ||
        ta === undefined ||
        sr === undefined ||
        sg === undefined ||
        sb === undefined ||
        sa === undefined ||
        br === undefined ||
        bg === undefined ||
        bb === undefined
      ) {
        expect.unreachable(name);
      }
      expect(apcaOn([tr, tg, tb, ta], [sr, sg, sb, sa], [br, bg, bb]), name).toBeCloseTo(lc, 6);
    }
  });

  test("APCA reads the published reference pairs", () => {
    expect(apcaLc([0x88, 0x88, 0x88], [0xff, 0xff, 0xff])).toBeCloseTo(63.06, 1);
    expect(apcaLc([0, 0, 0], [0xff, 0xff, 0xff])).toBeCloseTo(106.04, 1);
  });

  test("the card reports signed APCA readings against its drawn surface and marks shortfalls", () => {
    const declared: Record<string, string> = {
      "--tier-text": "90",
      "--tier-text-quiet": "75",
      "--tier-text-faint": "60",
      "--tier-slack": "0.05",
    };
    const claims = textClaims(
      ["--color-raised", "--color-text", "--color-text-quiet", "--color-text-faint", "--color-accent"],
      (name) => declared[name] ?? "",
    );
    expect(claims).toEqual({
      surface: "--color-raised",
      slack: 0.05,
      claims: [
        { token: "--color-text", tier: 90 },
        { token: "--color-text-quiet", tier: 75 },
        { token: "--color-text-faint", tier: 60 },
      ],
    });
    const drawn: Record<string, Rgba> = {
      "--color-raised": [0x22, 0x26, 0x2c, 1],
      "--color-text": [0xf0, 0xf2, 0xf5, 1],
      "--color-text-quiet": [0x55, 0x58, 0x5c, 1],
    };
    const [text, quiet, faint] = readLegibility(claims, (token) => drawn[token] ?? null);
    expect(text).toMatchObject({ kind: "measured", meetsTier: true });
    expect(quiet).toMatchObject({ kind: "measured", meetsTier: false });
    expect(faint).toMatchObject({ kind: "unmeasured", tier: 60 });
    expect(text?.kind === "measured" ? text.reached : null).toBeCloseTo(-96.18, 2);
  });
});
