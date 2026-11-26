// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Transaction } from "@codemirror/state";
import type { EditorView, ViewUpdate } from "@codemirror/view";

const typewriterMeasure = Symbol();

export function writingOffset(top: number, height: number, caret: number): number {
  const upper = top + height / 4;
  const lower = top + (3 * height) / 4;
  return caret < upper ? caret - upper : caret > lower ? caret - lower : 0;
}

export function followWriting(update: ViewUpdate): void {
  const { view, startState, state } = update;
  if (
    !view.hasFocus ||
    state.selection.ranges.length !== 1 ||
    !state.selection.main.empty ||
    !update.transactions.some(
      (tx) =>
        tx.annotation(Transaction.userEvent) === "select" ||
        tx.isUserEvent("input.type") ||
        tx.isUserEvent("delete"),
    )
  )
    return;
  const previous = update.changes.mapPos(startState.selection.main.head, -1);
  const head = state.selection.main.head;
  if (previous === head) return;
  view.requestMeasure({
    key: typewriterMeasure,
    read(now: EditorView) {
      if (
        now.state.doc !== state.doc ||
        !now.state.selection.eq(state.selection) ||
        now.compositionStarted
      )
        return 0;
      const from = now.coordsAtPos(previous, -1);
      const to = now.coordsAtPos(head);
      if (to === null || (from !== null && from.top < to.bottom && to.top < from.bottom)) return 0;
      const scroll = now.scrollDOM;
      return writingOffset(
        scroll.getBoundingClientRect().top + scroll.clientTop,
        scroll.clientHeight,
        (to.top + to.bottom) / 2,
      );
    },
    write(offset, now) {
      if (offset !== 0 && now.state.doc === state.doc && now.state.selection.eq(state.selection))
        now.scrollDOM.scrollTop += offset;
    },
  });
}
