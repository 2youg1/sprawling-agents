// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { browserRows, type Rows } from "./rows";
import { readonly, writable } from "./store";

export const MEASURES = ["gold", "silver", "wide"] as const;
export const INDENTATIONS = ["off", "display", "source"] as const;
export type WritingMeasure = (typeof MEASURES)[number];
export type WritingIndentation = (typeof INDENTATIONS)[number];

interface Writing {
  readonly measure: WritingMeasure;
  readonly indentation: WritingIndentation;
  readonly typewriter: boolean;
}

const MEASURE = "refrain.editor.measure";
const INDENTATION = "refrain.editor.indentation";
const TYPEWRITER = "refrain.editor.typewriter";

export function loadWriting(rows: Rows) {
  const held = writable<Writing>({
    measure: MEASURES.find((measure) => measure === rows.getItem(MEASURE)) ?? "silver",
    indentation: INDENTATIONS.find((indentation) => indentation === rows.getItem(INDENTATION)) ?? "off",
    typewriter: rows.getItem(TYPEWRITER) === "on",
  });
  return {
    held: readonly(held),
    measure(value: WritingMeasure) {
      rows.setItem(MEASURE, value);
      held.update((old) => ({ ...old, measure: value }));
    },
    indentation(value: WritingIndentation) {
      rows.setItem(INDENTATION, value);
      held.update((old) => ({ ...old, indentation: value }));
    },
    typewriter(value: boolean) {
      rows.setItem(TYPEWRITER, value ? "on" : "off");
      held.update((old) => ({ ...old, typewriter: value }));
    },
  };
}

let writing: ReturnType<typeof loadWriting> | undefined;

export function writingPreferences() {
  writing ??= loadWriting(browserRows());
  return writing;
}
