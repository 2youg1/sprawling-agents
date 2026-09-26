// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A page that asks several questions at once and hears nothing back
// used to put up one heading per question, each reading "this took too
// long and was cut", so a person could not tell which of them the city
// left unanswered without opening every fold.

import { describe, expect, test } from "bun:test";

import { noticeTitle } from "./notice_title";
import { QUERIES, keyOf } from "../../core/asking";

describe("the heading of a timeout", () => {
  test("names the question this page asked", () => {
    expect(noticeTitle("en", "E_TIMEOUT", keyOf(QUERIES.doctor))).toContain("doctor");
    expect(noticeTitle("zh", "E_TIMEOUT", keyOf(QUERIES.toolkits))).toContain("toolkits");
  });

  test("of a request the city cut keeps the code's own sentence", () => {
    expect(noticeTitle("en", "E_TIMEOUT", "api.zenmux.ai")).toBe("this took too long and was cut");
  });
});
