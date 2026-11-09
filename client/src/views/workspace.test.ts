// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { WORKBENCH } from "../core/workbench";
import { gridColumn, layoutOf, oneColumnOf } from "./workspace";

describe("layoutOf", () => {
  test("pins the talk box on the middle silver part across desktop tiers", () => {
    for (const tier of ["zen", "blend", "panorama"] as const) {
      const closed = layoutOf(tier, "closed", WORKBENCH);
      expect(closed.talk).toEqual([4, 10]);
      expect(closed.right).toBeNull();
    }
  });

  test("blend and panorama share the same side panes when the right side is closed", () => {
    const blend = layoutOf("blend", "closed", WORKBENCH);
    const panorama = layoutOf("panorama", "closed", WORKBENCH);
    expect(blend.world).toBe("beside");
    expect(panorama.world).toBe("beside");
    expect(blend.panes).toEqual(panorama.panes);
    expect(blend.panes).toEqual([
      { pane: "sessions", lines: [1, 4] },
      { pane: "commits", lines: [10, 13] },
    ]);
  });

  test("opening the right side moves talk left the same way in every tier", () => {
    for (const tier of ["zen", "blend", "panorama"] as const) {
      const open = layoutOf(tier, "open", WORKBENCH);
      expect(open.world).toBe("none");
      expect(open.panes).toEqual([]);
      expect(open.talk).toEqual([2, 8]);
      expect(open.right).toEqual([8, 13]);
    }
  });

  test("one-column panorama keeps the sheet and the band", () => {
    const layout = oneColumnOf("panorama", WORKBENCH);
    expect(layout.world).toBe("sheet");
    expect(layout.talk).toEqual([1, 13]);
    expect(layout.panes).toHaveLength(3);
  });

  test("gridColumn spells the CSS span", () => {
    expect(gridColumn([4, 10])).toBe("4 / 10");
  });
});
