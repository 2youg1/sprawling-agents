// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { isolateHistory } from "@codemirror/commands";
import { EditorSelection, type ChangeSpec } from "@codemirror/state";
import type { EditorView } from "@codemirror/view";

const QUOTE = /^( {0,3})>[ \t]?/u;

export function quoteMarkdown(view: EditorView): boolean {
  const { state } = view;
  const quoted = state.changeByRange((range) => {
    const first = state.doc.lineAt(range.from).number;
    const last = state.doc.lineAt(Math.max(range.from, range.to - 1)).number;
    const lines = [];
    for (let number = first; number <= last; number += 1) lines.push(state.doc.line(number));
    const remove = lines.some((line) => QUOTE.test(line.text)) &&
      lines.every((line) => QUOTE.test(line.text) || line.text.trim() === "");
    const edits: ChangeSpec[] = [];
    for (const line of lines) {
      const marker = QUOTE.exec(line.text);
      if (remove && marker !== null) {
        const keep = marker[1]?.length ?? 0;
        edits.push({ from: line.from + keep, to: line.from + marker[0].length });
      } else if (!remove && marker === null) {
        edits.push({ from: line.from, insert: "> " });
      }
    }
    const changes = state.changes(edits);
    return {
      changes,
      range: range.empty ? EditorSelection.cursor(changes.mapPos(range.head, 1)) : range.map(changes),
    };
  });
  if (quoted.changes.empty) return false;
  view.dispatch({
    ...quoted, scrollIntoView: true, annotations: isolateHistory.of("full"), userEvent: "input.format.quote",
  });
  return true;
}
