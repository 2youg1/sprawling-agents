// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What one stored row means: how the string a row holds reads back as
// a value of the record, and how a figure nobody stated stays out of
// storage. Which row is called what is `core/prefs.ts`'s, the one
// reader of these; every reading here drops what this build cannot
// read rather than repairing it, so the shipped posture stands for it.

import { STACK_SHAPE } from "./appearance";
import { langOf, type Lang } from "./lang";
import type { Rows } from "./rows";
import { sizingOf } from "./sizing";

// A stored word, or the posture this client ships with when the row is
// empty or holds a word this build no longer offers.
export function readOne<T extends string>(offered: readonly T[], raw: string | null, fallback: T): T {
  return offered.find((each) => each === raw) ?? fallback;
}

export function readBody(raw: string | null): number | null {
  const said = sizingOf(raw ?? "");
  return said.kind === "sized" ? said.px : null;
}

export function readStack(raw: string | null): string {
  return raw !== null && STACK_SHAPE.test(raw) ? raw : "";
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
