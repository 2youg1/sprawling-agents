// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";
import { Option, Schema } from "effect";

import { ContainerImage, ContainerLimits } from "../wire";

const image = `sha256:${"a".repeat(64)}`;

test("the generated image decoder refuses mutable tags and noncanonical digests", () => {
  const read = Schema.decodeOption(ContainerImage);
  expect(Option.isSome(read(image))).toBe(true);
  for (const invalid of ["alpine:latest", "sha256:abc", image.toUpperCase(), `${image}
`]) {
    expect(Option.isNone(read(invalid))).toBe(true);
  }
});

test("every generated container resource rejects the observed zero input", () => {
  const read = Schema.decodeOption(ContainerLimits);
  const valid = { image, user: 1, cpu_millis: 1, memory_bytes: 1, pids: 1 };
  expect(Option.isSome(read(valid))).toBe(true);
  for (const resource of ["user", "cpu_millis", "memory_bytes", "pids"]) {
    expect(Option.isNone(read({ ...valid, [resource]: 0 }))).toBe(true);
  }
});
