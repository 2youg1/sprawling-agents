// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";
import { createRoot, flush } from "solid-js";
import { pdfWindow } from "./pdf_window";

test("fast scrolling reserves preview capacity for visible pages and leads in the current direction", () => {
  const width = 950;
  const { model, dispose } = createRoot((dispose) => ({
    model: pdfWindow({
      count: 2_000,
      width: () => width,
      continuous: () => true,
      first: () => 1,
      turns: () => 1,
      picked: () => undefined,
    }),
    dispose,
  }));

  model.show(90, 91);
  model.placed(900);
  model.trackScroll(1_000);
  flush();
  expect(model.pages()).toEqual([88, 89, 90, 91, 92, 93, 94, 95]);
  expect([...model.prefetch()]).toEqual([92]);

  model.noteReady(90, width, "preview");
  model.noteReady(91, width, "preview");
  model.noteReady(92, width, "preview");
  flush();
  expect([...model.prefetch()]).toEqual([93, 94, 95]);

  model.trackScroll(900);
  flush();
  expect(model.pages()).toEqual([86, 87, 88, 89, 90, 91, 92, 93]);
  expect([...model.prefetch()]).toEqual([89, 88, 87]);

  model.settle();
  flush();
  expect([...model.prefetch()]).toEqual([]);
  dispose();
});
