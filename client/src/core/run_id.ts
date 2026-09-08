// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A run's identity: a uuid, held in the hyphenated lower-case form the
// Rust side's `Display` writes. Read in the two spellings `uuid::Uuid::
// parse_str` reads most (hyphenated and bare hex); the `urn:uuid:` and
// braced forms are not read, and no link in this product writes them.

import { Brand, Option } from "effect";

export type RunId = string & Brand.Brand<"RunId">;

const HYPHENATED =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const BARE = /^[0-9a-f]{32}$/i;

const brand = Brand.nominal<RunId>();

function hyphenate(bare: string): string {
  const cut = [8, 12, 16, 20];
  let out = "";
  let from = 0;
  for (const at of cut) {
    out += `${bare.slice(from, at)}-`;
    from = at;
  }
  return out + bare.slice(from);
}

// `RunId.option(raw)` is the only way to obtain one; the value it holds
// is canonical, so two ids for one run compare equal as strings.
export const RunId = {
  option(raw: string): Option.Option<RunId> {
    if (HYPHENATED.test(raw)) {
      return Option.some(brand(raw.toLowerCase()));
    }
    if (BARE.test(raw)) {
      return Option.some(brand(hyphenate(raw.toLowerCase())));
    }
    return Option.none();
  },
} as const;
