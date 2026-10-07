// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The version picker's look value is all a look draws, so these hold
// for any look: which version the list holds, and what choosing an
// entry hands back to the screen.

import { describe, expect, test } from "bun:test";

import { say } from "../../../core/lang";
import type { DocumentVersion } from "../../../wire";
import { B3Hash } from "../../../wire";
import { short } from "../reading";
import { againstLookOf } from "./against";

const V1 = B3Hash.make("1".repeat(64));
const V2 = B3Hash.make("2".repeat(64));

const OTHERS: readonly DocumentVersion[] = [
  { version: V1, bytes: 10, kept: true, source: "on_disk" },
  { version: V2, bytes: 12, kept: true, source: "on_disk" },
];

function picker(against: B3Hash | null): { readonly picked: (B3Hash | null)[]; readonly look: ReturnType<typeof againstLookOf> } {
  const picked: (B3Hash | null)[] = [];
  return { picked, look: againstLookOf(OTHERS, against, "en", (version) => {
    picked.push(version);
  }) };
}

function choose(look: ReturnType<typeof againstLookOf>, value: string): void {
  look.wire.onchange({ currentTarget: { value } });
}

describe("the version picker", () => {
  test("lists every other version by its short name after the entry that compares with nothing", () => {
    const { look } = picker(null);
    expect({ label: look.label, none: look.none, options: look.options }).toEqual({
      label: say("en", "format_compare_with"),
      none: say("en", "format_compare_none"),
      options: [
        { value: V1, label: short(V1) },
        { value: V2, label: short(V2) },
      ],
    });
  });

  test("holds the version compared with, or the empty value for none", () => {
    expect([picker(null).look.wire.value, picker(V2).look.wire.value]).toEqual(["", V2]);
  });

  test("hands back the version chosen, and nothing for the empty entry or a value it never listed", () => {
    const { picked, look } = picker(V1);
    choose(look, V2);
    choose(look, "");
    choose(look, "f".repeat(64));
    expect(picked).toEqual([V2, null, null]);
  });
});
