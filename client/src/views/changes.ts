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
import type { How, Lines } from "../wire";

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
