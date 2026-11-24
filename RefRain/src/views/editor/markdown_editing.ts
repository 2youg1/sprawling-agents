// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { markdown } from "@codemirror/lang-markdown";
import { syntaxTree } from "@codemirror/language";
import { StateEffect, StateField, Transaction } from "@codemirror/state";
import type { EditorState, Extension, Range } from "@codemirror/state";
import { Decoration, EditorView, ViewPlugin } from "@codemirror/view";
import type { DecorationSet, ViewUpdate } from "@codemirror/view";
import type { SyntaxNode } from "@lezer/common";
import { GFM } from "@lezer/markdown";
import type { MarkdownConfig } from "@lezer/markdown";
import {
  children,
  DiagramWidget,
  ImageWidget,
  MathWidget,
  TableWidget,
  webLink,
} from "./markdown_widgets";
import type { PreviewResource } from "./markdown_widgets";

export type MarkdownMode = "source" | "live";

export interface MarkdownOptions {
  readonly diagramLabel: string;
  readonly readResource?: PreviewResource | undefined;
}

const INLINE_MATH = { resolve: "InlineMath", mark: "MathMark" };
const DISPLAY_MATH = { resolve: "DisplayMath", mark: "MathMark" };

const MATH: MarkdownConfig = {
  defineNodes: ["InlineMath", "DisplayMath", "MathMark"],
  parseInline: [
    {
      name: "Math",
      after: "Escape",
      parse(context, next, at) {
        if (next !== 36 || context.char(at - 1) === 36) return -1;
        const display = context.char(at + 1) === 36;
        const end = at + (display ? 2 : 1);
        const before = context.char(at - 1);
        const after = context.char(end);
        const canOpen = after >= 0 && (display || after > 32);
        const canClose = before >= 0 && (display || before > 32) && !(after >= 48 && after <= 57);
        return context.addDelimiter(
          display ? DISPLAY_MATH : INLINE_MATH,
          at,
          end,
          canOpen,
          canClose,
        );
      },
    },
  ],
};

interface Viewport {
  readonly from: number;
  readonly to: number;
}

interface Drawing {
  readonly mode: MarkdownMode;
  readonly viewport: Viewport;
  readonly decorations: DecorationSet;
}

export const markdownMode = StateEffect.define<MarkdownMode>();
const scrolled = StateEffect.define<Viewport>();
const MARKER = Decoration.mark({ class: "cm-markdown-marker" });
const HIDDEN = Decoration.replace({});

const INLINE: Readonly<Record<string, { readonly tagName: string; readonly class?: string }>> = {
  Emphasis: { tagName: "em" },
  StrongEmphasis: { tagName: "strong" },
  Strikethrough: { tagName: "s" },
  InlineCode: { tagName: "code", class: "cm-markdown-code" },
};

const SOURCE_MARKERS = new Set([
  "HeaderMark",
  "EmphasisMark",
  "StrikethroughMark",
  "CodeMark",
  "LinkMark",
  "MathMark",
  "ListMark",
  "QuoteMark",
  "TableDelimiter",
]);

function selectionTouches(state: EditorState, node: SyntaxNode): boolean {
  return state.selection.ranges.some((range) => range.from <= node.to && range.to >= node.from);
}

function documentText(state: EditorState, node: SyntaxNode): string {
  return state.doc.sliceString(node.from, node.to);
}

function targetOf(state: EditorState, node: SyntaxNode): string | null {
  const url = node.getChild("URL");
  return url === null ? null : documentText(state, url).replace(/^<|>$/gu, "");
}

function terminal(
  state: EditorState,
  node: SyntaxNode,
  options: MarkdownOptions,
): Decoration | null {
  const name = node.name;
  if (name === "Image") {
    const url = targetOf(state, node);
    const start = node.firstChild?.to;
    const end = node.getChildren("LinkMark")[1]?.from;
    return url === null || start === undefined || end === undefined
      ? null
      : Decoration.replace({
          widget: new ImageWidget(url, state.doc.sliceString(start, end), options.readResource),
        });
  }
  if (name === "DisplayMath" || name === "InlineMath")
    return Decoration.replace({
      widget: new MathWidget(documentText(state, node)),
      block: name === "DisplayMath",
    });
  if (name === "Table") {
    const rows = [...children(node)]
      .filter((row) => row.name === "TableHeader" || row.name === "TableRow")
      .map((row) => row.getChildren("TableCell").map((cell) => documentText(state, cell).trim()));
    return Decoration.replace({
      widget: new TableWidget(rows, documentText(state, node)),
      block: true,
    });
  }
  if (name === "FencedCode") {
    const info = node.getChild("CodeInfo");
    const code = node.getChild("CodeText");
    if (info === null || code === null || documentText(state, info).trim() !== "mermaid")
      return null;
    return Decoration.replace({
      widget: new DiagramWidget(documentText(state, code), options.diagramLabel),
      block: true,
    });
  }
  return null;
}

function drawNode(
  out: Range<Decoration>[],
  state: EditorState,
  node: SyntaxNode,
  live: boolean,
  options: MarkdownOptions,
): void {
  if (node.from === node.to) return;
  const name = node.name;
  if (SOURCE_MARKERS.has(name)) {
    const hide = live && name !== "QuoteMark" && name !== "ListMark" && name !== "CodeMark";
    out.push((hide ? HIDDEN : MARKER).range(node.from, node.to));
    return;
  }
  if (live) {
    const widget = terminal(state, node, options);
    if (widget !== null) {
      out.push(widget.range(node.from, node.to));
      return;
    }
    const heading = /^(?:ATX|Setext)Heading([1-6])$/u.exec(name)?.[1];
    if (heading !== undefined) {
      const line = state.doc.lineAt(node.from);
      out.push(
        Decoration.line({
          class: `cm-markdown-heading cm-markdown-heading-${heading}`,
          attributes: { role: "heading", "aria-level": heading },
        }).range(line.from),
      );
    }
    const mark = INLINE[name];
    if (mark !== undefined) out.push(Decoration.mark(mark).range(node.from, node.to));
    if (name === "Link" || name === "Autolink") {
      const target = targetOf(state, node);
      const href = target === null ? null : webLink(target);
      if (href !== null)
        out.push(
          Decoration.mark({
            tagName: "a",
            class: "cm-markdown-link",
            attributes: { href, rel: "noopener noreferrer", target: "_blank", tabindex: "-1" },
          }).range(node.from, node.to),
        );
      const title = node.getChild("LinkTitle");
      if (title !== null) out.push(HIDDEN.range(title.from, title.to));
      const url = node.getChild("URL");
      if (url !== null && name === "Link") out.push(HIDDEN.range(url.from, url.to));
    }
  }
  for (const child of children(node)) {
    const hideCodeMark = live && name === "InlineCode" && child.name === "CodeMark";
    if (hideCodeMark) out.push(HIDDEN.range(child.from, child.to));
    else drawNode(out, state, child, live, options);
  }
}

function decorationSet(
  state: EditorState,
  mode: MarkdownMode,
  viewport: Viewport,
  options: MarkdownOptions,
): DecorationSet {
  const out: Range<Decoration>[] = [];
  syntaxTree(state).iterate({
    from: viewport.from,
    to: viewport.to,
    enter(ref) {
      if (ref.name === "Document") return true;
      const node = ref.node;
      drawNode(out, state, node, mode === "live" && !selectionTouches(state, node), options);
      return false;
    },
  });
  return Decoration.set(out, true);
}

export function markdownEditing(options: MarkdownOptions): Extension {
  const drawing = StateField.define<Drawing>({
    create(state) {
      const viewport = { from: 0, to: 0 };
      return {
        mode: "source",
        viewport,
        decorations: decorationSet(state, "source", viewport, options),
      };
    },
    update(old, tx) {
      if (tx.isUserEvent("input.type.compose")) {
        return {
          ...old,
          decorations: old.decorations.map(tx.changes),
          viewport: {
            from: tx.changes.mapPos(old.viewport.from),
            to: tx.changes.mapPos(old.viewport.to, 1),
          },
        };
      }
      let mode = old.mode;
      let viewport = {
        from: tx.changes.mapPos(old.viewport.from),
        to: tx.changes.mapPos(old.viewport.to, 1),
      };
      let redraw = false;
      for (const effect of tx.effects) {
        if (effect.is(markdownMode)) {
          mode = effect.value;
          redraw = true;
        } else if (effect.is(scrolled)) {
          viewport = effect.value;
          redraw = true;
        }
      }
      if (
        !redraw &&
        !tx.docChanged &&
        !tx.selection &&
        syntaxTree(tx.startState) === syntaxTree(tx.state)
      )
        return old;
      return { mode, viewport, decorations: decorationSet(tx.state, mode, viewport, options) };
    },
    provide: (field) => EditorView.decorations.from(field, (value) => value.decorations),
  });
  const viewport = ViewPlugin.fromClass(
    class {
      private current = true;
      private queued = false;
      constructor(readonly view: EditorView) {
        this.refresh();
      }
      update(update: ViewUpdate): void {
        if (update.viewportChanged) this.refresh();
      }
      refresh(): void {
        if (this.queued) return;
        this.queued = true;
        queueMicrotask(() => {
          this.queued = false;
          if (!this.current || this.view.compositionStarted) return;
          const want = this.view.viewport;
          const old = this.view.state.field(drawing).viewport;
          if (old.from !== want.from || old.to !== want.to)
            this.view.dispatch({
              effects: scrolled.of(want),
              annotations: Transaction.addToHistory.of(false),
            });
        });
      }
      destroy(): void {
        this.current = false;
      }
    },
  );
  return [
    markdown({ extensions: [GFM, MATH] }),
    drawing,
    viewport,
    EditorView.domEventHandlers({
      compositionend(_event, view) {
        queueMicrotask(() => view.plugin(viewport)?.refresh());
        return false;
      },
      click(event) {
        const target = event.target;
        if (!(target instanceof Element) || target.closest(".cm-markdown-link") === null)
          return false;
        if (!event.ctrlKey && !event.metaKey) event.preventDefault();
        return false;
      },
    }),
  ];
}
