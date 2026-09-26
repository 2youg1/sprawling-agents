// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";
import { Address } from "../wire";
import { removalOf } from "./removal";

// The page offers removal only where the city would carry it out: not
// City Hall, and not a building a run is working in.
test("removal is offered only for an idle building a person raised", () => {
  expect([
    removalOf(Address.make("lab"), 0),
    removalOf(Address.make("lab"), 2),
    removalOf(Address.make("hall"), 0),
  ]).toEqual(["offered", "busy", "hall"]);
});
