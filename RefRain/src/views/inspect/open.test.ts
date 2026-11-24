// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";
import { get } from "../../core/store";
import { Address } from "../../wire";
import { createInspector, rightItem, sharedInspector, type RightItem } from "./open";

test("independent inspectors keep picks and cancellation local", () => {
  const live = get(sharedInspector.items);
  const tall = createInspector();
  const wide = createInspector();
  const a: RightItem = {
    kind: "document",
    building: Address.make("lab"),
    path: "a.md",
    version: null,
  };
  const b: RightItem = { ...a, path: "b.md" };
  tall.showItem(a);
  wide.showItem(b);
  tall.closeRight();
  expect(rightItem(get(tall.items))).toBeNull();
  expect(rightItem(get(wide.items))).toEqual(b);
  expect(get(sharedInspector.items)).toBe(live);
});
