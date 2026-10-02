// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Which place a change of reading carries over. Only a Markdown preview
// has blocks to carry a place to and from; the source and the diff are
// one editor, so a switch between them carries nothing.

import type { Reading } from "./reading";

// Where the reader stands in the editor: the first line in view, as an
// editor offset in the baseline. The caret is not the place, because a
// reader scrolls away from it to read, and the switch keeps what they
// read at the top.
export interface EditorPlace {
  readonly top: number;
}

export type Carried =
  // The preview opens with the block holding this baseline offset.
  | { readonly kind: "preview_at"; readonly offset: number }
  // The editor scrolls so this baseline byte is its first line.
  | { readonly kind: "editor_at"; readonly byte: number }
  | { readonly kind: "nothing" };

const EDITOR: ReadonlySet<Reading> = new Set(["source", "diff"]);

export function carried(from: Reading, to: Reading, editor: EditorPlace, previewTop: number): Carried {
  if (EDITOR.has(from) && to === "preview") return { kind: "preview_at", offset: editor.top };
  if (from === "preview" && EDITOR.has(to)) return { kind: "editor_at", byte: previewTop };
  return { kind: "nothing" };
}
