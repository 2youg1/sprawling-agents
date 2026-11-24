// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { diff, unifiedMergeView } from "@codemirror/merge";
import { search, searchKeymap } from "@codemirror/search";
import {
  Annotation,
  ChangeSet,
  Compartment,
  EditorState,
  Text,
  Transaction,
} from "@codemirror/state";
import type { Extension } from "@codemirror/state";
import {
  EditorView,
  type ViewUpdate,
  drawSelection,
  highlightActiveLine,
  highlightSpecialChars,
  keymap,
  lineNumbers,
  rectangularSelection,
} from "@codemirror/view";

import type { EditorChange } from "../../core/document_pos";
import { markdownEditing, markdownMode } from "./markdown_editing";
import type { MarkdownMode, MarkdownOptions } from "./markdown_editing";

export interface TextRange {
  readonly from: number;
  readonly to: number;
}

export interface ContextSelection {
  readonly range: TextRange;
  readonly x: number;
  readonly y: number;
}

export interface EditorSetup {
  readonly parent: HTMLElement;

  readonly text: string;
  readonly changes: readonly EditorChange[];

  readonly label: string;

  readonly phrases: Readonly<Record<string, string>>;
  readonly onEdit: () => void;
  readonly onSave: () => void;
  readonly markdown?: MarkdownOptions | undefined;
  readonly onMarkdownMode?: (mode: MarkdownMode) => void;
  readonly onSelection?: (range: TextRange | null) => void;
  readonly onContextSelection?: (selection: ContextSelection) => void;
}

export interface Editing {
  readonly changes: () => EditorChange[];
  readonly hasChanges: () => boolean;

  readonly editedSinceSent: () => boolean | null;
  readonly text: () => string;
  readonly slice: (range: TextRange) => string;

  readonly sent: () => void;

  readonly landed: () => void;
  readonly unsent: () => void;

  readonly rebase: (theirs: string) => void;
  readonly rebaseChange: (theirs: string, change: EditorChange) => void;
  readonly follow: (theirs: string) => void;

  readonly diffing: (on: boolean) => void;
  readonly setMarkdown: (mode: MarkdownMode) => void;
  readonly typewriter: (on: boolean) => void;
  readonly readOnly: (on: boolean) => void;
  readonly composing: () => boolean;

  readonly top: () => number;
  readonly reveal: (at: number) => void;
  readonly revealRange: (range: TextRange) => void;
  readonly selection: () => TextRange | null;

  readonly scrollTop: (at: number) => void;

  readonly focus: () => void;
  readonly toBaseline: (at: number) => number;
  readonly fromBaseline: (at: number) => number;
  readonly measure: () => void;
  readonly destroy: () => void;
}

const Moved = Annotation.define<boolean>();

function textOf(text: string): Text {
  return Text.of(text.split("\n"));
}

function between(from: string, to: string): ChangeSet {
  return ChangeSet.of(
    diff(from, to).map((change) => ({
      from: change.fromA,
      to: change.toA,
      insert: to.slice(change.fromB, change.toB),
    })),
    from.length,
  );
}

function listed(changes: ChangeSet): EditorChange[] {
  const out: EditorChange[] = [];
  changes.iterChanges((fromA, toA, _fromB, _toB, inserted) => {
    out.push({ from: fromA, to: toA, insert: inserted.toString() });
  });
  return out;
}

function compared(original: string): Extension {
  return unifiedMergeView({
    original,
    mergeControls: false,
    gutter: true,
    syntaxHighlightDeletions: false,
  });
}

function common(label: string, phrases: Readonly<Record<string, string>>): Extension[] {
  return [
    lineNumbers(),
    highlightSpecialChars(),
    EditorView.lineWrapping,
    EditorState.phrases.of(phrases),
    EditorView.contentAttributes.of({ "aria-label": label }),
  ];
}

export function openEditor(setup: EditorSetup): Editing {
  let base = setup.text;
  let composed = ChangeSet.of(
    setup.changes.map((each) => ({ ...each })),
    base.length,
  );
  let sentText: string | null = null;
  let sinceSent: ChangeSet | null = null;
  let showing = false;
  let composing = false;
  let awaiting: MarkdownMode | null = null;
  let typewriter = false;
  let alive = true;
  const merge = new Compartment();
  const locked = new Compartment();
  const measureInput =
    performance.getEntriesByName("sprawling:measure-editor-input", "mark").length > 0;

  function selected(view: EditorView): TextRange | null {
    const { from, to, empty } = view.state.selection.main;
    return empty ? null : { from, to };
  }

  function setMarkdown(mode: MarkdownMode): void {
    if (setup.markdown === undefined) return;
    if (composing || view.compositionStarted) {
      awaiting = mode;
      return;
    }
    view.dispatch({
      effects: [markdownMode.of(mode), view.scrollSnapshot()],
      annotations: Transaction.addToHistory.of(false),
    });
    setup.onMarkdownMode?.(mode);
  }

  let selectionHeld = false;
  function applyInput(update: ViewUpdate, measured = false): void {
    const started = measured ? performance.now() : 0;
    const changed = update.docChanged;
    const selectedOrChanged = update.selectionSet || changed;
    if (selectedOrChanged && setup.onSelection !== undefined) {
      const range = selected(update.view);
      if (range !== null || selectionHeld) setup.onSelection(range);
      selectionHeld = range !== null;
    }
    if (
      typewriter &&
      !composing &&
      selectedOrChanged &&
      update.transactions.some((tx) => tx.annotation(Transaction.userEvent) !== undefined)
    ) {
      queueMicrotask(() => {
        if (!alive || view.compositionStarted) return;
        const at = view.state.selection.main.head;
        view.dispatch({ effects: EditorView.scrollIntoView(at, { y: "center" }) });
      });
    }
    const selectionReady = measured ? performance.now() : 0;
    const edited = changed && !update.transactions.some((each) => each.annotation(Moved) === true);
    if (edited) {
      composed = composed.compose(update.changes);
      sinceSent = sinceSent?.compose(update.changes) ?? null;
    }
    const composedAt = measured ? performance.now() : 0;
    if (edited) setup.onEdit();
    if (measured) {
      const ended = performance.now();
      performance.measure("sprawling:editor_input_handler", {
        start: started,
        end: ended,
        detail: {
          selectionMs: selectionReady - started,
          composeMs: composedAt - selectionReady,
          notifyMs: ended - composedAt,
          transactions: update.transactions.length,
        },
      });
    }
  }

  function receiveInput(update: ViewUpdate): void {
    applyInput(
      update,
      measureInput && update.transactions.some((transaction) => transaction.isUserEvent("input")),
    );
  }

  const view = new EditorView({
    parent: setup.parent,
    state: EditorState.create({
      doc: composed.apply(textOf(base)),
      extensions: [
        ...common(setup.label, setup.phrases),
        history(),
        EditorState.allowMultipleSelections.of(true),
        drawSelection(),
        rectangularSelection(),
        highlightActiveLine(),
        search({ top: true }),
        keymap.of([
          {
            key: "Mod-s",
            preventDefault: true,
            run: (editor) => {
              if (!editor.composing) setup.onSave();
              return true;
            },
          },
          ...searchKeymap,
          ...historyKeymap,
          ...defaultKeymap,
        ]),
        merge.of([]),
        locked.of(EditorState.readOnly.of(false)),
        setup.markdown === undefined ? [] : markdownEditing(setup.markdown),
        EditorView.domEventHandlers({
          compositionstart() {
            composing = true;
            return false;
          },
          compositionend() {
            composing = false;
            requestAnimationFrame(() => {
              if (!alive || awaiting === null) return;
              const mode = awaiting;
              awaiting = null;
              setMarkdown(mode);
            });
            return false;
          },
          contextmenu(event, editor) {
            const range = selected(editor);
            if (range === null || setup.onContextSelection === undefined) return false;
            const at = editor.coordsAtPos(range.to);
            const fromKeyboard = event.clientX === 0 && event.clientY === 0;
            setup.onContextSelection({
              range,
              x: fromKeyboard ? (at?.left ?? 0) : event.clientX,
              y: fromKeyboard ? (at?.bottom ?? 0) : event.clientY,
            });
            return true;
          },
          keydown(event, editor) {
            if (event.key !== "ContextMenu" && !(event.shiftKey && event.key === "F10"))
              return false;
            const range = selected(editor);
            if (range === null || setup.onContextSelection === undefined) return false;
            const at = editor.coordsAtPos(range.to);
            setup.onContextSelection({ range, x: at?.left ?? 0, y: at?.bottom ?? 0 });
            return true;
          },
        }),
        EditorView.updateListener.of(receiveInput),
      ],
    }),
  });

  const reoriginate = (): void => {
    if (showing) view.dispatch({ effects: merge.reconfigure(compared(base)) });
  };

  const move = (
    theirs: string,
    then: (onTheirs: ChangeSet) => ChangeSet,
    onTheirs = between(base, theirs),
  ): void => {
    const kept = then(onTheirs);
    view.dispatch({
      changes: composed.invert(textOf(base)).compose(onTheirs).compose(kept),
      annotations: [Moved.of(true), Transaction.addToHistory.of(false)],
    });
    base = theirs;
    composed = kept;
    reoriginate();
  };

  const clamp = (at: number): number => Math.min(Math.max(at, 0), view.state.doc.length);

  return {
    changes: () => listed(composed),
    hasChanges: () => !composed.empty,
    editedSinceSent: () => (sinceSent === null ? null : !sinceSent.empty),
    text: () => view.state.doc.toString(),
    slice: ({ from, to }) => view.state.doc.sliceString(from, to),
    sent: () => {
      sentText = view.state.doc.toString();
      sinceSent = ChangeSet.empty(sentText.length);
    },
    landed: () => {
      if (sentText !== null && sinceSent !== null) {
        base = sentText;
        composed = sinceSent;
        reoriginate();
      }
      sentText = null;
      sinceSent = null;
    },
    unsent: () => {
      sentText = null;
      sinceSent = null;
    },
    rebase: (theirs) => {
      move(theirs, (onTheirs) => composed.map(onTheirs));
    },
    rebaseChange: (theirs, change) => {
      move(theirs, (onTheirs) => composed.map(onTheirs), ChangeSet.of(change, base.length));
    },
    follow: (theirs) => {
      move(theirs, () => ChangeSet.empty(theirs.length));
    },
    diffing: (on) => {
      showing = on;
      view.dispatch({ effects: merge.reconfigure(on ? compared(base) : []) });
    },
    setMarkdown,
    typewriter: (on) => {
      typewriter = on;
    },
    readOnly: (on) => {
      if (view.state.readOnly !== on)
        view.dispatch({ effects: locked.reconfigure(EditorState.readOnly.of(on)) });
      const spoken = String(on);
      if (view.contentDOM.getAttribute("aria-readonly") !== spoken)
        view.contentDOM.setAttribute("aria-readonly", spoken);
    },
    composing: () => composing || view.compositionStarted,
    top: () => view.lineBlockAtHeight(view.scrollDOM.scrollTop).from,
    reveal: (at) => {
      const position = clamp(at);
      view.dispatch({
        selection: { anchor: position },
        effects: EditorView.scrollIntoView(position, { y: "start", yMargin: 24 }),
      });
    },
    revealRange: (range) => {
      const from = clamp(range.from);
      const to = clamp(range.to);
      view.dispatch({
        selection: { anchor: from, head: to },
        effects: EditorView.scrollIntoView(from, { y: "center" }),
      });
    },
    selection: () => selected(view),
    scrollTop: (at) => {
      const position = Math.min(Math.max(at, 0), view.state.doc.length);
      view.dispatch({ effects: EditorView.scrollIntoView(position, { y: "start" }) });
    },
    focus: () => {
      view.focus();
    },
    toBaseline: (at) => composed.invertedDesc.mapPos(at),
    fromBaseline: (at) => composed.mapPos(at),
    measure: () => {
      view.requestMeasure();
    },
    destroy: () => {
      alive = false;
      view.destroy();
    },
  };
}

export function openComparison(
  parent: HTMLElement,
  texts: { readonly from: string; readonly to: string },
  label: string,
  phrases: Readonly<Record<string, string>>,
): { readonly destroy: () => void } {
  const view = new EditorView({
    parent,
    state: EditorState.create({
      doc: texts.to,
      extensions: [...common(label, phrases), EditorState.readOnly.of(true), compared(texts.from)],
    }),
  });
  view.contentDOM.setAttribute("aria-readonly", "true");
  return {
    destroy: () => {
      view.destroy();
    },
  };
}
