// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { FAMILY_SHAPE, STACK_SHAPE } from "./appearance";
import { langOf, type Lang } from "./lang";
import type { Rows } from "./rows";
import { sizingOf } from "./sizing";

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

export function readFamily(raw: string | null): string {
  return raw !== null && FAMILY_SHAPE.test(raw) ? raw : "";
}

export function readMany<T extends string>(
  offered: readonly T[],
  raw: string | null,
): readonly T[] {
  const said = new Set((raw ?? "").split(",").map((each) => each.trim()));
  return offered.filter((each) => said.has(each));
}

export function readAtLeast(raw: string | null, floor: number): number | null {
  if (!/^-?[0-9]+$/.test(raw ?? "")) return null;
  const said = Number(raw);
  return said >= floor ? said : null;
}

export function readLang(raw: string | null, fallback: string): Lang {
  return raw === "en" || raw === "zh" ? raw : langOf(fallback);
}

export function writeFigure(rows: Rows, row: string, figure: number | null): void {
  if (figure === null) rows.removeItem(row);
  else rows.setItem(row, String(figure));
}

export function writeNames(rows: Rows, row: string, names: readonly string[]): void {
  if (names.length === 0) rows.removeItem(row);
  else rows.setItem(row, names.join(","));
}
