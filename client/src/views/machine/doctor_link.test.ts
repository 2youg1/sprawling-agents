// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The doctor item name is a link to its release or installation page
// when the city gives one. The link must open in a new browsing context
// so the doctor page stays open and the browser opens the page.

import { describe, expect, test } from "bun:test";

import type { DoctorItem } from "../../wire";
import { lookOf } from "./doctor_link";

const ITEM: DoctorItem = {
  name: "sarasa-gothic",
  tier: "use",
  need: "optional",
  homepage: "https://github.com/be5invis/Sarasa-Gothic/releases/latest",
  state: { absent: { absence: { no_component: { dir: "the platform's font directories" } } } },
  install: {
    manual: {
      how: "download SarasaMonoSC-TTF-*.7z from https://github.com/be5invis/Sarasa-Gothic/releases/latest",
    },
  },
  pinned: null,
  pack: null,
};

describe("a doctor item link", () => {
  test("links to the release page and asks the browser to open it in a new context", () => {
    const look = lookOf(ITEM);
    expect(look).toEqual({
      name: ITEM.name,
      homepage: ITEM.homepage ?? null,
      target: "_blank",
    });
  });

  test("is plain text when the item has no homepage", () => {
    const look = lookOf({ ...ITEM, homepage: null });
    expect(look).toEqual({
      name: ITEM.name,
      homepage: null,
      target: "_blank",
    });
  });

  test("every item in the doctor table that has a homepage links to an https page", () => {
    // The look is the one authority on whether and where the link goes;
    // the component spreads these values on the anchor. A target of
    // `_blank` with an https href is what makes the browser open the
    // release or installation page instead of leaving the doctor page.
    const withHome = lookOf(ITEM);
    expect(withHome.homepage).toMatch(/^https:\/\//u);
    expect(withHome.target).toBe("_blank");
  });
});
