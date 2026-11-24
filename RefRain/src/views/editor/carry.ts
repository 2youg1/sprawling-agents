// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Reading } from "./reading";

export interface EditorPlace {
  readonly top: number;
}

export type Carried =
  | { readonly kind: "preview_at"; readonly offset: number }
  | { readonly kind: "editor_at"; readonly byte: number }
  | { readonly kind: "nothing" };

const EDITOR: ReadonlySet<Reading> = new Set(["source", "live", "diff"]);

export function carried(
  from: Reading,
  to: Reading,
  editor: EditorPlace,
  previewTop: number,
): Carried {
  if (EDITOR.has(from) && to === "preview") return { kind: "preview_at", offset: editor.top };
  if (from === "preview" && EDITOR.has(to)) return { kind: "editor_at", byte: previewTop };
  return { kind: "nothing" };
}
