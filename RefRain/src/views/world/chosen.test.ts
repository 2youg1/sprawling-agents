// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";
import { get } from "../../core/store";
import { galleryUi, ui } from "../../ui";
import { PICKED } from "../gallery/benched";
import { commitSelection, commitStateOf, pickCommit } from "./chosen";

test("a stand's selected commit stays out of the other stand and the live workspace", () => {
  expect(PICKED).toBeDefined();
  if (PICKED === undefined) return;
  const live = ui();
  const previous = get(commitSelection);
  const first = galleryUi(live);
  const second = galleryUi(live);
  pickCommit(PICKED, commitStateOf(first));
  expect(get(commitStateOf(first))).toEqual({
    room: PICKED.actor,
    run: PICKED.run,
    oid: PICKED.oid,
  });
  expect(get(commitStateOf(second))).toBeNull();
  expect(get(commitSelection)).toBe(previous);
});
