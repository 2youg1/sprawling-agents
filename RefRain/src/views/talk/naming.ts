// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import { Option, Schema } from "effect";

import { say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import { MAYOR, roomOf } from "../../core/route";
import { Address } from "../../wire";

export function called(address: Address, mayor: string | null | undefined, lang: Lang): string {
  if (address !== MAYOR) return roomOf(address);
  return mayor ?? say(lang, "nav_mayor");
}


export type Dispatcher =
  | { readonly kind: "person" }
  | { readonly kind: "city" }
  | { readonly kind: "resident"; readonly address: Address };

const readAddress = Schema.decodeOption(Address);


export function dispatcherOf(by: string | null | undefined): Dispatcher | null {
  if (by === undefined || by === null) return null;
  if (by === "person") return { kind: "person" };
  if (by === "city") return { kind: "city" };
  const address = residentAt(by);
  return address === null ? null : { kind: "resident", address };
}


export function residentAt(text: string | null | undefined): Address | null {
  if (text === undefined || text === null) return null;
  return Option.getOrNull(readAddress(text));
}
