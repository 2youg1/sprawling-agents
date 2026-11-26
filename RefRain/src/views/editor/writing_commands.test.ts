// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { EditorState, Transaction } from "@codemirror/state";

import { markdownEditing } from "./markdown_editing";
import { planUndo, undoPoolExtension, undoRows } from "./undo_pool";
import { paragraphIndents, typedIndent } from "./writing_indent";
import { writingOffset } from "./writing_scroll";

const FULL = "\u3000";
const prose = "这是一段中文正文。";
const structure = [
  "# 标题",
  "",
  "- 列表",
  "  - 子项",
  "",
  "| 名称 | 值 |",
  "| --- | --- |",
  "| 项目 | 1 |",
  "",
  "```mermaid",
  "flowchart TD",
  "  A --> B",
  "```",
  "",
  "> 引用",
  "> 第二行",
  "",
  "下面是标题",
  "==========",
  "",
  FULL,
  "",
].join("\n");
const markdown = () => markdownEditing({ diagramLabel: "" });

describe("writing indentation", () => {
  test("only fills nonempty top-level prose and preserves all structural bytes", () => {
    const original = structure + "\n" + prose + "\n\n" + FULL + "已有一个全角空格。\n";
    const state = EditorState.create({ doc: original, extensions: markdown() });
    const settled = state.update({ changes: paragraphIndents(state) }).state;
    expect(settled.doc.toString()).toBe(
      structure + "\n" + FULL.repeat(2) + prose + "\n\n" + FULL.repeat(2) + "已有一个全角空格。\n",
    );
    expect(paragraphIndents(settled)).toEqual([]);
  });

  test("typed prose receives its prefix in the typing transaction, paste does not", () => {
    const base = EditorState.create({ extensions: [markdown(), typedIndent(() => true)] });
    const input = { changes: { from: 0, insert: "你" }, selection: { anchor: 1 } };
    const typed = base.update({ ...input, userEvent: "input.type" }).state;
    expect(typed.doc.toString()).toBe(FULL.repeat(2) + "你");
    expect(typed.selection.main.head).toBe(3);
    const pasted = base.update({ ...input, userEvent: "input.paste" }).state;
    expect(pasted.doc.toString()).toBe("你");
  });

  test("typing inside a quote and a fenced diagram keeps the literal bytes", () => {
    for (const text of ["> ", "```mermaid\n", "# ", "- ", "| "]) {
      const base = EditorState.create({
        doc: text, selection: { anchor: text.length }, extensions: [markdown(), typedIndent(() => true)],
      });
      const next = base.update({
        changes: { from: text.length, insert: "甲" }, selection: { anchor: text.length + 1 },
        userEvent: "input.type",
      });
      expect(next.state.doc.toString()).toBe(text + "甲");
    }
  });

  test("withdraws only the generated prefix when the active paragraph becomes a heading", () => {
    const base = EditorState.create({ extensions: [markdown(), typedIndent(() => true)] });
    const typed = base.update({ changes: { from: 0, insert: "标题" }, userEvent: "input.type" }).state;
    expect(typed.doc.toString()).toBe(FULL.repeat(2) + "标题");
    const title = typed.update({ changes: { from: typed.doc.length, insert: "\n===" }, userEvent: "input.type" }).state;
    expect(title.doc.toString()).toBe("标题\n===");
  });
});

describe("selective undo", () => {
  test("preserves a later independent edit and supports choosing both", () => {
    const base = EditorState.create({ doc: "甲 乙 丙", extensions: undoPoolExtension });
    const older = base.update({ changes: { from: 0, to: 1, insert: "较长" } }).state;
    const current = older.update({ changes: { from: 5, to: 6, insert: "丁" } }).state;
    const rows = undoRows(current);
    const earliest = rows.at(-1);
    expect(earliest).toBeDefined();
    if (earliest === undefined) return;
    const partial = planUndo(current, new Set([earliest.id]));
    expect(partial.kind === "ready" ? partial.changes.apply(current.doc).toString() : partial.kind)
      .toBe("甲 乙 丁");
    const all = planUndo(current, new Set(rows.map((row) => row.id)));
    expect(all.kind === "ready" ? all.changes.apply(current.doc).toString() : all.kind).toBe("甲 乙 丙");
  });

  test("requires an overlapping later edit and leaves no plan after an external revision", () => {
    const base = EditorState.create({ doc: "交付。", extensions: undoPoolExtension });
    const older = base.update({ changes: { from: 2, insert: "下周一" } }).state;
    const current = older.update({ changes: { from: 2, to: 5, insert: "周一上午" } }).state;
    const rows = undoRows(current);
    const earliest = rows.at(-1);
    expect(earliest).toBeDefined();
    if (earliest === undefined) return;
    expect(planUndo(current, new Set([earliest.id])).kind).toBe("conflict");
    const all = planUndo(current, new Set(rows.map((row) => row.id)));
    expect(all.kind === "ready" ? all.changes.apply(current.doc).toString() : all.kind).toBe("交付。");
    const received = current.update({
      changes: { from: 0, insert: "新" }, annotations: Transaction.addToHistory.of(false),
    }).state;
    expect(undoRows(received)).toEqual([]);
    expect(planUndo(received, new Set([earliest.id])).kind).toBe("empty");
  });

  test("reversing the later edit releases an earlier edit of the restored text", () => {
    const base = EditorState.create({ doc: "交付。", extensions: undoPoolExtension });
    const first = base.update({ changes: { from: 2, insert: "下周一" } }).state;
    const second = first.update({ changes: { from: 2, to: 5, insert: "周一上午" } }).state;
    const [newer, older] = undoRows(second);
    expect(newer).toBeDefined();
    expect(older).toBeDefined();
    if (newer === undefined || older === undefined) return;
    const reverseNewer = planUndo(second, new Set([newer.id]));
    expect(reverseNewer.kind).toBe("ready");
    if (reverseNewer.kind !== "ready") return;
    const restored = second.update({ changes: reverseNewer.changes, userEvent: "input.revert" }).state;
    expect(restored.doc.toString()).toBe("交付下周一。");
    const reverseOlder = planUndo(restored, new Set([older.id]));
    expect(reverseOlder.kind === "ready" ? reverseOlder.changes.apply(restored.doc).toString() : reverseOlder.kind)
      .toBe("交付。");
  });
});

test("typewriter measures from the scrolling viewport and moves only to the nearest boundary", () => {
  expect([50, 100, 150, 300, 350, 390].map((caret) => writingOffset(50, 400, caret)))
    .toEqual([-100, -50, 0, 0, 0, 40]);
});
