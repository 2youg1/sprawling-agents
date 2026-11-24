// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";
import type { AxError } from "../wire";
import { diagnosticToml } from "./diagnostic";

const refused: AxError = {
  code: "E_INVALID_ARGS",
  action: "answer an approval",
  subject: "fixture-approval",
  recovery: "this item is not waiting; the inbox lists the ones that are",
  retry: "no",
  nearby: [],
};

test("includes a stable approval recovery in a safe diagnostic", () => {
  const text = diagnosticToml(refused);
  expect(text).toStartWith('code = "E_INVALID_ARGS"');
  expect(text).toContain(`recovery = "${refused.recovery}"`);
});

test("does not export unstructured recovery text", () => {
  const recovery = "Unstructured input from the provider";
  expect(diagnosticToml({ ...refused, recovery })).not.toContain(recovery);
});
