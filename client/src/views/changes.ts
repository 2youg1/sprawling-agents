// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The three folds a file-change row states: what happened to the file,
// how much of it moved, and the seven characters of an object id that
// tell two commits apart in a row. `changes.svelte` draws them and a
// working-tree list states the same three facts about the same
// `FileChange`, so the folds live here rather than being translated
// twice (`parts/glyph.ts` is the companion module of the same kind).

import { fill, say } from "../core/lang";
import type { Lang } from "../core/lang";
import type { HunksAnswer, How, Lines } from "../wire";

export function howWord(lang: Lang, how: How): string {
  if (typeof how !== "string") {
    return fill(say(lang, "change_renamed"), { from: how.renamed.from });
  }
  switch (how) {
    case "added":
      return say(lang, "change_added");
    case "modified":
      return say(lang, "change_modified");
    case "deleted":
      return say(lang, "change_deleted");
  }
}

export function linesWord(lang: Lang, lines: Lines): string {
  if (typeof lines === "string") {
    return say(lang, "change_binary");
  }
  return `+${String(lines.counted.added)} −${String(lines.counted.removed)}`;
}

export function shortOid(oid: string): string {
  return oid.slice(0, 7);
}

// One line of a patch as a reader points at it. `number` is the line's
// place in the patch text and is what keys it on the page.
export type Numbered =
  | { readonly kind: "head"; readonly number: number; readonly text: string }
  | { readonly kind: "hunk"; readonly number: number; readonly text: string; readonly folded: number }
  | { readonly kind: "context"; readonly number: number; readonly text: string; readonly old: Place; readonly new: Place }
  | { readonly kind: "added"; readonly number: number; readonly text: string; readonly new: Place }
  | { readonly kind: "removed"; readonly number: number; readonly text: string; readonly old: Place }
  | { readonly kind: "withheld"; readonly number: number; readonly reason: string };

// A line's number in one of the two files, or `null` once a withheld
// line has made the count unknowable until the next hunk header.
export type Place = number | null;

// A hunk header's two starts and the new side's length: `@@ -a[,b] +c[,d] @@`.
const HUNK = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,(\d+))? @@/u;

// Walks the patch in its own order, withheld lines in their places. The
// hunk headers are the only authority on where a line sits in either
// file, so a withheld line - whose side the patch no longer says -
// leaves both counts unknown until the next header restores them, and
// the page draws a blank rather than a number that may be one off.
export function numbered(patch: HunksAnswer): Numbered[] {
  const withheld: Numbered[] = patch.withheld.map((each) => ({ kind: "withheld", number: each.number, reason: each.reason }));
  const shown: readonly (Numbered | { readonly number: number; readonly text: string })[] = [...patch.lines, ...withheld];
  const walk = [...shown].sort((a, b) => a.number - b.number);
  const drawn: Numbered[] = [];
  let old: Place = null;
  let now: Place = null;
  let end: number | null = null;
  for (const line of walk) {
    if ("kind" in line) {
      drawn.push(line);
      old = null;
      now = null;
      continue;
    }
    const { number, text } = line;
    const header = HUNK.exec(text);
    if (header !== null) {
      const start = Number(header[2]);
      const length = header[3] === undefined ? 1 : Number(header[3]);
      drawn.push({ kind: "hunk", number, text, folded: Math.max(0, start - (end ?? 1)) });
      old = Number(header[1]);
      now = start;
      end = start + length;
    } else if (end === null) {
      drawn.push({ kind: "head", number, text });
    } else if (text.startsWith("+")) {
      drawn.push({ kind: "added", number, text: text.slice(1), new: now });
      now = now === null ? null : now + 1;
    } else if (text.startsWith("-")) {
      drawn.push({ kind: "removed", number, text: text.slice(1), old });
      old = old === null ? null : old + 1;
    } else {
      drawn.push({ kind: "context", number, text: text.slice(1), old, new: now });
      old = old === null ? null : old + 1;
      now = now === null ? null : now + 1;
    }
  }
  return drawn;
}

// What a chosen patch line puts in the composer: where it is, then the
// line itself as a quote. A removed line lives only in the older tree,
// so its place names that tree's commit.
export function quoteLine(patch: HunksAnswer, line: CodeLine): string {
  switch (line.kind) {
    case "added":
    case "context":
      return quoted(line.new === null ? patch.path : `${patch.path}:${String(line.new)}`, line.text);
    case "removed":
      return quoted(line.old === null ? patch.path : `${patch.path}@${shortOid(patch.oid_a)}:${String(line.old)}`, line.text);
  }
}

// The lines a person can point at: the ones that are code in one tree
// or the other.
export type CodeLine = Extract<Numbered, { readonly kind: "added" | "context" | "removed" }>;

function quoted(place: string, text: string): string {
  return `${place}\n> ${text}\n`;
}
