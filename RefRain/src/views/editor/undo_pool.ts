// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { invertedEffects, isolateHistory } from "@codemirror/commands";
import { ChangeSet, StateEffect, StateField, Transaction, type ChangeSpec, type EditorState, type Text } from "@codemirror/state";
import type { EditorView } from "@codemirror/view";

const POOL_ACTIONS = 64;
const POOL_UNITS = 4 * 1024 * 1024;
const TYPING_GAP = 500;
const PREVIEW_LENGTH = 120;
const PREVIEW_SPANS = 32;

interface Entry {
  readonly id: number;
  readonly time: number;
  readonly typing: boolean;
  readonly forward: ChangeSet;
  readonly inverse: ChangeSet;
  readonly line: number;
  readonly snippet: string;
  readonly units: number;
  readonly reversed: boolean;
}

interface Journal {
  readonly next: number;
  readonly entries: readonly Entry[];
}

interface Mark {
  readonly ids: ReadonlySet<number>;
  readonly reversed: boolean;
}

const mark = StateEffect.define<Mark>();

type Span = readonly [number, number];

function spans(set: ChangeSet): Span[] {
  const out: Span[] = [];
  set.iterChangedRanges((from, to) => out.push([from, to]));
  return out;
}

function rangeContact(a: Span, b: Span, adjacent: boolean): boolean {
  const [from, to] = a;
  const [start, end] = b;
  if (adjacent) return from <= end && start <= to;
  if (from === to || start === end) return from <= end && start <= to;
  return from < end && start < to;
}

export function changesContact(a: ChangeSet, b: ChangeSet, adjacent = false): boolean {
  const left = spans(a);
  const right = spans(b);
  let at = 0;
  for (const span of left) {
    while (at < right.length && (right[at]?.[1] ?? 0) < span[0]) at += 1;
    for (let index = at; index < right.length; index += 1) {
      const other = right[index];
      if (other === undefined || other[0] > span[1]) break;
      if (rangeContact(span, other, adjacent)) return true;
    }
  }
  return false;
}

function entryOf(tx: Transaction, id: number, previous: Entry | undefined): Entry {
  const forward = previous?.forward.compose(tx.changes) ?? tx.changes;
  const inverse = tx.changes.invert(tx.startState.doc);
  let units = previous?.units ?? 0;
  let line = previous?.line ?? 1;
  let snippet = previous?.snippet ?? "";
  tx.changes.iterChanges((from, to, _afterFrom, _afterTo, inserted) => {
    units += to - from + inserted.length;
    if (snippet !== "") return;
    line = tx.startState.doc.lineAt(from).number;
    snippet = inserted.length > 0
      ? inserted.sliceString(0, PREVIEW_LENGTH)
      : tx.startState.doc.sliceString(from, Math.min(to, from + PREVIEW_LENGTH));
  });
  return {
    id,
    time: tx.annotation(Transaction.time) ?? 0,
    typing: tx.isUserEvent("input.type") || tx.isUserEvent("delete"),
    forward,
    inverse: previous === undefined ? inverse : inverse.compose(previous.inverse),
    line,
    snippet,
    units,
    reversed: false,
  };
}

function bounded(entries: readonly Entry[]): readonly Entry[] {
  let count = 0;
  let units = 0;
  for (let index = entries.length - 1; index >= 0; index -= 1) {
    units += entries[index]?.units ?? 0;
    if (units > POOL_UNITS || count === POOL_ACTIONS) break;
    count += 1;
  }
  return entries.slice(entries.length - count);
}

export const undoJournal = StateField.define<Journal>({
  create: () => ({ next: 0, entries: [] }),
  update(old, tx) {
    const marks = tx.effects.filter((effect) => effect.is(mark)).map((effect) => effect.value);
    const boundary = tx.annotation(isolateHistory);
    if (!tx.docChanged && marks.length === 0) {
      const last = old.entries.at(-1);
      if (boundary !== undefined && last?.typing)
        return { ...old, entries: [...old.entries.slice(0, -1), { ...last, typing: false }] };
      return old;
    }
    if (tx.docChanged && tx.annotation(Transaction.addToHistory) === false)
      return { next: old.next, entries: [] };
    const entries = marks.length === 0 ? old.entries : old.entries.map((entry) => {
      const latest = [...marks].reverse().find((named) => named.ids.has(entry.id));
      return latest === undefined ? entry : { ...entry, reversed: latest.reversed };
    });
    if (!tx.docChanged) return { ...old, entries };
    const last = entries.at(-1);
    const time = tx.annotation(Transaction.time) ?? 0;
    const typing = tx.isUserEvent("input.type") || tx.isUserEvent("delete");
    if (
      typing && last?.typing && !last.reversed && boundary !== "before" && boundary !== "full" &&
      (tx.isUserEvent("input.type.compose") || time - last.time < TYPING_GAP) &&
      changesContact(last.inverse, tx.changes, true)
    ) {
      return { ...old, entries: bounded([...entries.slice(0, -1), entryOf(tx, last.id, last)]) };
    }
    return { next: old.next + 1, entries: bounded([...entries, entryOf(tx, old.next, undefined)]) };
  },
});

export const undoPoolExtension = [
  undoJournal,
  invertedEffects.of((tx) =>
    tx.effects.flatMap((effect) =>
      effect.is(mark) ? [mark.of({ ...effect.value, reversed: !effect.value.reversed })] : [],
    ),
  ),
];

export function undoRows(state: EditorState): readonly Entry[] {
  return state.field(undoJournal).entries.filter((entry) => !entry.reversed).reverse();
}

export type UndoPoolPlan =
  | { readonly kind: "empty" }
  | { readonly kind: "conflict"; readonly later: Entry }
  | {
      readonly kind: "ready";
      readonly changes: ChangeSet;
      readonly doc: Text;
      readonly journal: Journal;
    };

function changedText(changes: ChangeSet, original: Text): ChangeSet {
  const kept: ChangeSpec[] = [];
  let identical = false;
  changes.iterChanges((from, to, _start, _end, insert) => {
    if (to - from === insert.length && original.slice(from, to).eq(insert)) identical = true;
    else kept.push({ from, to, insert });
  });
  return identical ? ChangeSet.of(kept, original.length) : changes;
}

export function planUndo(state: EditorState, picked: ReadonlySet<number>): UndoPoolPlan {
  const journal = state.field(undoJournal);
  let remaining = picked.size;
  let carried = ChangeSet.empty(state.doc.length);
  let planned = carried;
  let observed = state.doc;
  let later: Entry | undefined;
  for (const entry of [...journal.entries].reverse()) {
    if (remaining === 0) break;
    if (!picked.has(entry.id) || entry.reversed) {
      carried = entry.forward.compose(carried);
      later ??= entry;
      observed = entry.inverse.apply(observed);
      continue;
    }
    if (later !== undefined && changesContact(entry.inverse, carried)) {
      carried = changedText(carried, observed);
      if (changesContact(entry.inverse, carried)) {
        const dependency = dependencyFor(journal.entries, entry, picked) ?? later;
        return { kind: "conflict", later: dependency };
      }
    }
    const mapped = entry.inverse.map(carried);
    carried = carried.map(entry.inverse, true);
    planned = planned.compose(mapped);
    observed = entry.inverse.apply(observed);
    remaining -= 1;
  }
  if (remaining !== 0 || planned.empty || planned.apply(state.doc).eq(state.doc))
    return { kind: "empty" };
  return { kind: "ready", changes: planned, journal, doc: state.doc };
}

function dependencyFor(entries: readonly Entry[], entry: Entry, picked: ReadonlySet<number>): Entry | undefined {
  let inverse = entry.inverse;
  for (const next of entries) {
    if (next.id <= entry.id) continue;
    if (picked.has(next.id) && !next.reversed) {
      inverse = inverse.map(next.forward);
      continue;
    }
    if (changesContact(inverse, next.forward)) return next;
    inverse = inverse.map(next.forward);
  }
  return undefined;
}

export function applyUndo(view: EditorView, plan: UndoPoolPlan, picked: ReadonlySet<number>): boolean {
  if (
    plan.kind !== "ready" || view.state.doc !== plan.doc ||
    view.state.field(undoJournal) !== plan.journal || view.state.readOnly || view.compositionStarted
  ) return false;
  view.dispatch({
    changes: plan.changes,
    effects: mark.of({ ids: picked, reversed: true }),
    annotations: isolateHistory.of("full"),
    userEvent: "input.revert",
  });
  return true;
}

interface PreviewSpan {
  readonly line: number;
  readonly before: string;
  readonly after: string;
}

export function undoPreview(plan: Extract<UndoPoolPlan, { kind: "ready" }>) {
  const spans: PreviewSpan[] = [];
  let count = 0;
  plan.changes.iterChanges((from, to, _start, _end, inserted) => {
    count += 1;
    if (spans.length === PREVIEW_SPANS) return;
    const line = plan.doc.lineAt(from);
    spans.push({
      line: line.number,
      before: plan.doc.sliceString(from, Math.min(to, from + PREVIEW_LENGTH)),
      after: inserted.sliceString(0, PREVIEW_LENGTH),
    });
  });
  return { count, spans };
}
