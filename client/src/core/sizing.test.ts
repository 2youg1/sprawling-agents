// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The size box refuses exactly what `wire::BodyPx` refuses: a size under
// the floor. There is no ceiling, so a size the old range refused is read.

import { expect, test } from "bun:test";

import { BODY_PX } from "../wire";
import { sizingOf } from "./sizing";

test("a size under the floor is refused and any whole size from it up is read", () => {
  expect(sizingOf(String(BODY_PX.min - 1))).toEqual({ kind: "refused" });
  expect(sizingOf(String(BODY_PX.min))).toEqual({ kind: "sized", px: BODY_PX.min });
  expect(sizingOf(" 24 ")).toEqual({ kind: "sized", px: 24 });
  expect(sizingOf("96")).toEqual({ kind: "sized", px: 96 });
  expect(sizingOf("15.5")).toEqual({ kind: "refused" });
  expect(sizingOf("")).toEqual({ kind: "cleared" });
});
