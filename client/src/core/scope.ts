// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The seam between the two spellings of one scope.
//
// A city has one notion of what a halt applies to and two ways of
// writing it down, by a decision recorded in `kernel::event::scope`:
// the Ledger holds `city`, `building:<addr>`, `workshop:<addr>`,
// because that is what every history already written holds, while a
// frame carries the tagged shape a sender would type. Both reach this
// client - answers in the frame shape, records in the Ledger shape -
// and this file is the one place they meet.
//
// Everything else in the client works in the frame shape, so nothing
// else compares strings. That is the defect this file exists to close:
// a page comparing a bare address against `building:<addr>` reported
// every shut building as running, and nothing failed.

import { Option, Schema } from "effect";

import { Address } from "../wire";
import type { HaltScope } from "../wire";

// The three words, as `kernel::event::scope` spells them, and the
// character that separates a kind from the address it names.
//
// `CITY` is exported because the two spellings agree on this one word:
// the Ledger writes `city` and a frame writes `city`, so a view that
// halts the whole city and a view that asks whether it is halted read
// one constant rather than each spelling it again.
export const CITY = "city";
const BUILDING = "building";
const WORKSHOP = "workshop";
const AT = ":";

// The address grammar, which `kernel::Address::parse` states for both
// ends. Decoding rather than asserting: the text after the separator is
// an address only if it is one, and a scope naming something else is a
// record this build cannot read.
const readAddress = Schema.decodeOption(Address);

// A scope as the Ledger spelled it, or null for a word this build does
// not know.
//
// Null rather than a default: a client that read an unknown scope as
// the city would report the whole city shut, and one that read it as
// nothing would show a stopped building as running. Neither is better
// than saying nothing.
export function scopeOf(spelled: string | null): HaltScope | null {
  if (spelled === null) return null;
  if (spelled === CITY) return CITY;
  const cut = spelled.indexOf(AT);
  if (cut < 0) return null;
  const kind = spelled.slice(0, cut);
  const addr = Option.getOrNull(readAddress(spelled.slice(cut + 1)));
  if (addr === null) return null;
  if (kind === BUILDING) return { building: addr };
  if (kind === WORKSHOP) return { workshop: addr };
  return null;
}

// Whether two scopes name the same thing.
//
// Structural, because a scope is a value: the tagged shape has no
// identity of its own, so two answers naming one building are two
// objects.
export function sameScope(left: HaltScope, right: HaltScope): boolean {
  if (left === CITY || right === CITY) return left === right;
  if ("building" in left && "building" in right) return left.building === right.building;
  if ("workshop" in left && "workshop" in right) return left.workshop === right.workshop;
  return false;
}

// Whether a list of shut scopes holds one building.
export function buildingIsShut(halted: readonly HaltScope[], addr: Address): boolean {
  return halted.some((held) => held !== CITY && "building" in held && held.building === addr);
}

// Whether a list of shut scopes holds the city itself.
export function cityIsShut(halted: readonly HaltScope[]): boolean {
  return halted.includes(CITY);
}
