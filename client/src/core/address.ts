// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A canonical relative path inside the city, ported grammar for grammar
// from `kernel::address::Address::parse`. Until the generator emits the
// wire schema, this is the client's one authority for what an address is.

import { Brand } from "effect";

export type Address = string & Brand.Brand<"Address">;

// Unicode category Cc, which is what `char::is_control` answers.
const CONTROL = /\p{Cc}/u;

function violation(raw: string): string | undefined {
  if (raw === "") {
    return "empty";
  }
  if (raw.startsWith("/")) {
    return "absolute";
  }
  if (raw.includes("\\")) {
    return "backslash";
  }
  if (raw.includes(":")) {
    return "colon";
  }
  if (CONTROL.test(raw)) {
    return "control";
  }
  for (const segment of raw.split("/")) {
    if (segment === "") {
      return "empty_segment";
    }
    if (segment === "." || segment === "..") {
      return "dot_segment";
    }
  }
  return undefined;
}

// `Address.option(raw)` is the only way to obtain one; an illegal string
// answers `None`, which is the `Result` the Rust side returns.
export const Address = Brand.refined<Address>(
  (raw) => violation(raw) === undefined,
  (raw) => Brand.error(`address: ${violation(raw) ?? "ok"}`),
);
