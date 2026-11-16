// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { RunId } from "../wire";
import { NO_TAIL, appended } from "./live_output";

const run = RunId.make("00000000-0000-0000-0000-000000000007");

describe("a running command's tail", () => {
  test("keeps its newest lines under the cap and counts the ones it dropped", () => {
    const flooded = ["a\nb\n", "c\nd\n", "e"].reduce(
      (tail, text) => appended(tail, { run, stream: "out", text }, 3),
      NO_TAIL,
    );
    const warned = appended(flooded, { run, stream: "err", text: "oops\n" }, 3);
    expect(warned).toEqual({ out: "c\nd\ne", err: "oops\n", cut: 2 });
  });
});
