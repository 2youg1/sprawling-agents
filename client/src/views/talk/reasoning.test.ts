// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";

import { lengthOf } from "./reasoning";

test("reasoning length is UTF-8 bytes, never a character or token guess", () => {
  // ASCII is one byte per character; CJK is three, so "字" alone is three
  // bytes and must not be reported as one character.
  expect(lengthOf("en", "abc")).toBe("3 bytes");
  expect(lengthOf("zh", "字")).toBe("3 字节");
  expect(lengthOf("en", "café")).toBe("5 bytes");
});
