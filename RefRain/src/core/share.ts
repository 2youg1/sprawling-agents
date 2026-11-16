// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A share of a whole, held in billionths.
//
// The city carries a share as a count of billionths
// (`kernel::share::WHOLE_PPB`) rather than as a fraction, so two
// readings compare the same way on every machine and no ledger payload
// holds a float. This is where that scale becomes a number a person
// reads, and the only place the whole is spelled: the client used to
// divide by 1e7 in one file and 1e9 in another, which is one fact with
// two homes and a third waiting to join them.

const WHOLE_PPB = 1_000_000_000;

// A share as a fraction of the whole, zero to one. For a paint, where
// the number is applied to a width rather than read.
export function fraction(ppb: number): number {
  return ppb / WHOLE_PPB;
}

// A share as a whole percentage, zero to one hundred. Rounded, because
// a percentage a person reads carries no more of the share than the two
// digits in front of the sign; `fraction` keeps the rest.
export function percent(ppb: number): number {
  return Math.round(ppb / (WHOLE_PPB / 100));
}
