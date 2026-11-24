// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { isServer } from "@solidjs/web";
import { expect, test } from "bun:test";
import { scopedUi } from "./ui";

test("the Solid runtime under test is the browser build", () => {
  expect(isServer).toBe(false);
});

test("starting a client root does not access context before that root exists", () => {
  expect(scopedUi()).toBeUndefined();
});
