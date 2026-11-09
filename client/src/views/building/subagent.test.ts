// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";

import { choiceOf, tiersOf } from "./subagent";

const RULES = `confidential = false
[workspace_models]
allowed = [{ endpoint = "house", model = "ordinary" }]
max_effort = "high"
[workspace_models.choice]
endpoint = "house"
model = "ordinary"
effort = "high"
[workspace_models.recommendations.easy]
endpoint = "house"
model = "ordinary"
effort = "low"
[workspace_models.recommendations.medium]
endpoint = "house"
model = "ordinary"
effort = "medium"
[workspace_models.recommendations.hard]
endpoint = "house"
model = "strong"
effort = "high"
`;

test("a direct choice and the three shortcuts are read from the rules", () => {
  expect(choiceOf(RULES)).toEqual({ endpoint: "house", model: "ordinary", effort: "high" });
  expect(tiersOf(RULES).hard).toEqual({ endpoint: "house", model: "strong", effort: "high" });
  expect(choiceOf("confidential = false\n")).toBeUndefined();
});

test("a single-quoted choice and a commented header still read", () => {
  const text = `confidential = false
[workspace_models.choice] # selected
endpoint = 'house'
model = 'vendor"model'
effort = 'high'
`;
  expect(choiceOf(text)).toEqual({
    endpoint: "house",
    model: 'vendor"model',
    effort: "high",
  });
});
