// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import type { AxError } from "../../wire";
import { answered, refused, sent, waited } from "./saving";

function refusal(code: AxError["code"]): AxError {
  return { action: "replace a document", code, nearby: [], recovery: "read the file again", retry: "no", subject: "MAYOR.md" };
}

describe("a settings card's save", () => {
  test("is saved only when the city answers with another version", () => {
    const saving = sent("v1");
    expect(answered(saving, "v1")).toEqual(saving);
    expect(answered(saving, "v2")).toEqual({ kind: "saved" });
  });

  test("keeps the draft when the document moved underneath it", () => {
    const conflict = refusal("E_VERSION_CONFLICT");
    expect(refused(sent("v1"), conflict)).toEqual({ kind: "refused", error: conflict });
  });

  test("is not refused by a refusal of something else", () => {
    expect(refused(sent("v1"), refusal("E_BUSY"))).toEqual(sent("v1"));
  });

  test("says it cannot tell after the patience, and still settles on a later receipt", () => {
    const unsure = waited(sent("v1"));
    expect(unsure).toEqual({ kind: "unverified", from: "v1" });
    expect(answered(unsure, "v2")).toEqual({ kind: "saved" });
  });
});
