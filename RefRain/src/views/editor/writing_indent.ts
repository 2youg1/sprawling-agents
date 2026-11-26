// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { ensureSyntaxTree, language } from "@codemirror/language";
import { isolateHistory } from "@codemirror/commands";
import {
  EditorState,
  StateEffect,
  StateField,
  type ChangeSpec,
  type Text,
  type Transaction,
} from "@codemirror/state";
import type { SyntaxNode } from "@lezer/common";

const SPACE = "\u3000";
const MAX_INDENT = 2;

interface AddedIndent {
  readonly from: number;
  readonly insert: string;
}

const written = StateEffect.define<AddedIndent | null>();

function carried(indent: AddedIndent, tx: Transaction): AddedIndent | null {
  const end = indent.from + indent.insert.length;
  let touched = false;
  tx.changes.iterChangedRanges((from, to) => {
    if (from < end && to > indent.from) touched = true;
  });
  if (touched) return null;
  const from = tx.changes.mapPos(indent.from, 1);
  return tx.newDoc.sliceString(from, from + indent.insert.length) === indent.insert
    ? { ...indent, from }
    : null;
}

const recentIndent = StateField.define<AddedIndent | null>({
  create: () => null,
  update(old, tx) {
    for (const effect of tx.effects) if (effect.is(written)) return effect.value;
    return old === null || !tx.docChanged ? old : carried(old, tx);
  },
});

export function ordinaryParagraph(node: SyntaxNode): boolean {
  if (node.name !== "Paragraph" || node.parent?.name !== "Document") return false;
  const only = node.firstChild;
  return (
    only === null ||
    only.from !== node.from ||
    only.to !== node.to ||
    (only.name !== "Image" && only.name !== "DisplayMath")
  );
}

function prefix(line: string): number | null {
  const leading = line.length - line.replace(/^\u3000*/u, "").length;
  return /^\s/u.test(line.slice(leading)) ? null : leading;
}

export function missingIndent(line: string): string {
  if (line.trim() === "") return "";
  const leading = prefix(line);
  return leading === null ? "" : SPACE.repeat(Math.max(0, MAX_INDENT - leading));
}

export function paragraphIndents(state: EditorState): readonly ChangeSpec[] {
  const parser = state.facet(language)?.parser;
  if (parser === undefined) return [];
  const edits: ChangeSpec[] = [];
  for (
    let node = parser.parse(state.doc.toString()).topNode.firstChild;
    node !== null;
    node = node.nextSibling
  ) {
    if (!ordinaryParagraph(node)) continue;
    const line = state.doc.lineAt(node.from);
    const insert = missingIndent(line.text);
    if (insert !== "") edits.push({ from: line.from, insert });
  }
  return edits;
}

export function freshParagraph(doc: Text, at: number): number | null {
  const line = doc.lineAt(at);
  if (
    at !== line.to ||
    !/^\u3000*[“‘「『《〈（]*$/u.test(line.text) ||
    (line.number > 1 && !/^[ \t]*$/u.test(doc.line(line.number - 1).text))
  )
    return null;
  return line.from;
}

export function finishParagraph(state: EditorState, start: number): AddedIndent | null {
  const line = state.doc.lineAt(start);
  if (line.from !== start || !/^\u3000*[“‘「『《〈（]*\p{L}/u.test(line.text)) return null;
  const tree = ensureSyntaxTree(state, line.to, 15);
  if (tree === null) return null;
  let node: SyntaxNode | null = tree.resolveInner(line.from, 1);
  while (node !== null && !ordinaryParagraph(node)) node = node.parent;
  if (node === null || state.doc.lineAt(node.from).from !== start) return null;
  const insert = missingIndent(line.text);
  return insert === "" ? null : { from: start, insert };
}

export function paragraphInput(indent: AddedIndent) {
  return { changes: indent, effects: written.of(indent), sequential: true };
}

function withdrawn(tx: Transaction): AddedIndent | null {
  const old = tx.startState.field(recentIndent);
  if (old === null) return null;
  const doc = tx.startState.doc;
  const first = doc.lineAt(old.from);
  const next = doc.line(Math.min(doc.lines, first.number + 1));
  if (!tx.changes.touchesRange(first.from, next.to)) return null;
  const mapped = carried(old, tx);
  if (mapped === null) return null;
  const line = tx.newDoc.lineAt(mapped.from);
  const to = tx.newDoc.line(Math.min(tx.newDoc.lines, line.number + 1)).to;
  const tree = ensureSyntaxTree(tx.state, to, 15);
  if (tree === null) return null;
  let node = tree.resolveInner(mapped.from, 1);
  while (node.parent !== null && node.parent.name !== "Document") node = node.parent;
  return node.name !== "Document" && !ordinaryParagraph(node) ? mapped : null;
}

export function typedIndent(enabled: () => boolean) {
  return [
    recentIndent,
    EditorState.transactionFilter.of((tx) => {
      if (!enabled() || !tx.docChanged || tx.isUserEvent("input.type.compose")) return tx;
      if (tx.isUserEvent("input") || tx.isUserEvent("delete")) {
        const undo = withdrawn(tx);
        if (undo !== null)
          return [
            tx,
            {
              changes: { from: undo.from, to: undo.from + undo.insert.length },
              effects: written.of(null),
              sequential: true,
            },
          ];
      }
      if (!tx.isUserEvent("input.type")) return tx;
      const { selection, doc } = tx.startState;
      if (selection.ranges.length !== 1 || !selection.main.empty) return tx;
      const start = freshParagraph(doc, selection.main.head);
      if (start === null) return tx;
      const indent = finishParagraph(tx.state, tx.changes.mapPos(start, -1));
      return indent === null
        ? tx
        : [
            tx,
            {
              ...paragraphInput(indent),
              annotations: isolateHistory.of("before"),
            },
          ];
    }),
  ];
}
