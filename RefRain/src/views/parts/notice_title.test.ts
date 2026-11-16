// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { noticeTitle, recoveryWords } from "./notice_title";
import { AxCode } from "../../wire";
import table from "../../lang.json";
import { QUERIES, keyOf } from "../../core/asking";

describe("the heading of a timeout", () => {
  test("names the question this page asked", () => {
    expect(noticeTitle("en", "E_TIMEOUT", keyOf(QUERIES.doctor))).toContain("doctor");
    expect(noticeTitle("zh", "E_TIMEOUT", keyOf(QUERIES.toolkits))).toContain("toolkits");
  });

  test("of a request the city cut keeps the code's own sentence", () => {
    expect(noticeTitle("en", "E_TIMEOUT", "api.example.test")).toBe(
      "this took too long and was cut",
    );
  });
});

describe("the next step under a refusal", () => {
  const CITY = "attach a provider on the settings page and pick a model for this tag";

  test("is the page's own sentence in the reader's language", () => {
    expect(recoveryWords("zh", "E_MODEL_UNCHOSEN", CITY)).toBe(table.recover_e_model_unchosen.zh);
  });

  test("is the city's sentence for a code this build has no word for", () => {
    expect(recoveryWords("zh", "E_NOT_YET_NAMED", CITY)).toBe(CITY);
  });

  test("has a word for every code the wire can carry", () => {
    const missing = AxCode.literals.filter(
      (code) => !Object.hasOwn(table, `recover_${code.toLowerCase()}`),
    );
    expect(missing).toEqual([]);
  });
});
