// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What one stored row means: how the string a row holds reads back as
// a value of the record, and how a figure nobody stated stays out of
// storage. Which row is called what is `core/prefs.ts`'s, the one
// reader of these; every reading here drops what this build cannot
// read rather than repairing it, so the shipped posture stands for it.

import { FAMILY_SHAPE, STACK_SHAPE } from "./appearance";
import { langOf, type Lang } from "./lang";
import type { Rows } from "./rows";
import { sizingOf } from "./sizing";

// A stored word, or the posture this client ships with when the row is
// empty or holds a word this build no longer offers.
export function readOne<T extends string>(
  offered: readonly T[],
  raw: string | null,
  fallback: T,
): T {
  return offered.find((each) => each === raw) ?? fallback;
}

export function readBody(raw: string | null): number | null {
  const said = sizingOf(raw ?? "");
  return said.kind === "sized" ? said.px : null;
}

export function readStack(raw: string | null): string {
  return raw !== null && STACK_SHAPE.test(raw) ? raw : "";
}

// One fallback slot's family name, or nothing when the row is empty or
// holds something a family name may not: a name that could not be a
// name is not repaired into one.
export function readFamily(raw: string | null): string {
  return raw !== null && FAMILY_SHAPE.test(raw) ? raw : "";
}

// A stored row of words read back as the subset this build offers, in
// the order `offered` declares them: a word this build dropped, a repeat
// and an empty row all leave nothing behind.
export function readMany<T extends string>(
  offered: readonly T[],
  raw: string | null,
): readonly T[] {
  const said = new Set((raw ?? "").split(",").map((each) => each.trim()));
  return offered.filter((each) => said.has(each));
}

// A whole number at or above a floor, or nothing: the row and the box of
// a ratio or a size read their domain the same way.
export function readAtLeast(raw: string | null, floor: number): number | null {
  if (!/^-?[0-9]+$/.test(raw ?? "")) return null;
  const said = Number(raw);
  return said >= floor ? said : null;
}

export function readLang(raw: string | null, fallback: string): Lang {
  return raw === "en" || raw === "zh" ? raw : langOf(fallback);
}

// A size or an opacity the person has not stated is absent from storage
// too, so the stylesheet's own figure keeps its one home in the theme.
export function writeFigure(rows: Rows, row: string, figure: number | null): void {
  if (figure === null) rows.removeItem(row);
  else rows.setItem(row, String(figure));
}

// The words a row holds, in the order the caller lists them; none of them
// is the same as no row at all.
export function writeNames(rows: Rows, row: string, names: readonly string[]): void {
  if (names.length === 0) rows.removeItem(row);
  else rows.setItem(row, names.join(","));
}
