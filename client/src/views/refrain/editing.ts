// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// RefRain's editor: one CodeMirror view over the editor text of one
// version, and the changes made on it since that version (client-SPEC
// 7N, 12-23). The page never holds a second copy of what the person
// typed: the view's document is the draft, and the change set from the
// baseline is what is kept, saved and moved to another version.
//
// This module and everything it imports are one lazy chunk; the first
// screen of the client never downloads an editor.

import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { diff, unifiedMergeView } from "@codemirror/merge";
import { search, searchKeymap } from "@codemirror/search";
import { Annotation, ChangeSet, Compartment, EditorState, Text } from "@codemirror/state";
import type { Extension } from "@codemirror/state";
import {
  EditorView,
  highlightActiveLine,
  highlightSpecialChars,
  keymap,
  lineNumbers,
} from "@codemirror/view";

import type { EditorChange } from "../../core/document_pos";

export interface EditorSetup {
  readonly parent: HTMLElement;
  // The baseline's editor text, and a kept draft's changes on it.
  readonly text: string;
  readonly changes: readonly EditorChange[];
  // The file's name, which is the text box's accessible name.
  readonly label: string;
  // CodeMirror's own words (the find panel), in the page's language.
  readonly phrases: Readonly<Record<string, string>>;
  readonly onEdit: () => void;
  readonly onSave: () => void;
}

export interface Editing {
  // The changes from the baseline to the text now, in document order.
  readonly changes: () => EditorChange[];
  // Whether the text moved since the last `sent`, and `null` when no
  // save is in flight.
  readonly editedSinceSent: () => boolean | null;
  readonly text: () => string;
  // A save left with the text as it is now.
  readonly sent: () => void;
  // That save landed: its text is the baseline, and what was typed
  // since is the draft. Or it did not: the draft is everything again.
  readonly landed: () => void;
  readonly unsent: () => void;
  // The city holds `theirs` now. `rebase` carries the draft over onto
  // it; `follow` drops the draft and shows it.
  readonly rebase: (theirs: string) => void;
  readonly follow: (theirs: string) => void;
  // Shows the draft against its baseline, or stops.
  readonly diffing: (on: boolean) => void;
  readonly readOnly: (on: boolean) => void;
  readonly composing: () => boolean;
  // Editor positions: where the cursor is, the first visible one, and
  // the baseline's position for a draft's one and back.
  readonly cursor: () => number;
  readonly top: () => number;
  readonly reveal: (at: number) => void;
  // Puts the keyboard in the editor.
  readonly focus: () => void;
  readonly toBaseline: (at: number) => number;
  readonly fromBaseline: (at: number) => number;
  readonly measure: () => void;
  readonly destroy: () => void;
}

// A change the page made to the document rather than the person: it is
// not part of the draft, which the page sets itself after it.
const Moved = Annotation.define<boolean>();

function textOf(text: string): Text {
  return Text.of(text.split("\n"));
}

// The changes that turn `from` into `to`, as one change set on `from`.
function between(from: string, to: string): ChangeSet {
  return ChangeSet.of(
    diff(from, to).map((change) => ({ from: change.fromA, to: change.toA, insert: to.slice(change.fromB, change.toB) })),
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
  return unifiedMergeView({ original, mergeControls: false, gutter: true, syntaxHighlightDeletions: false });
}

// What both of RefRain's views are made of: line numbers, wrapped
// lines, control characters drawn rather than hidden (a text file is
// shown literally), and the page's words.
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
  let composed = ChangeSet.of(setup.changes.map((each) => ({ ...each })), base.length);
  let sentText: string | null = null;
  let sinceSent: ChangeSet | null = null;
  let showing = false;
  const merge = new Compartment();
  const locked = new Compartment();

  const view = new EditorView({
    parent: setup.parent,
    state: EditorState.create({
      doc: composed.apply(textOf(base)),
      extensions: [
        ...common(setup.label, setup.phrases),
        history(),
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
        EditorView.updateListener.of((update) => {
          if (!update.docChanged || update.transactions.some((each) => each.annotation(Moved) === true)) return;
          composed = composed.compose(update.changes);
          sinceSent = sinceSent?.compose(update.changes) ?? null;
          setup.onEdit();
        }),
      ],
    }),
  });

  const reoriginate = (): void => {
    if (showing) view.dispatch({ effects: merge.reconfigure(compared(base)) });
  };

  // Puts the text the city now holds under the view: the view moves from
  // the draft to `theirs` with `then` applied, as one change, so the
  // cursor and the undo history follow it rather than starting over.
  const move = (theirs: string, then: (onTheirs: ChangeSet) => ChangeSet): void => {
    const onTheirs = between(base, theirs);
    const kept = then(onTheirs);
    view.dispatch({ changes: composed.invert(textOf(base)).compose(onTheirs).compose(kept), annotations: Moved.of(true) });
    base = theirs;
    composed = kept;
    reoriginate();
  };

  return {
    changes: () => listed(composed),
    editedSinceSent: () => (sinceSent === null ? null : !sinceSent.empty),
    text: () => view.state.doc.toString(),
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
    follow: (theirs) => {
      move(theirs, () => ChangeSet.empty(theirs.length));
    },
    diffing: (on) => {
      showing = on;
      view.dispatch({ effects: merge.reconfigure(on ? compared(base) : []) });
    },
    readOnly: (on) => {
      view.dispatch({ effects: locked.reconfigure(EditorState.readOnly.of(on)) });
      view.contentDOM.setAttribute("aria-readonly", String(on));
    },
    composing: () => view.composing,
    cursor: () => view.state.selection.main.head,
    top: () => view.lineBlockAtHeight(view.scrollDOM.scrollTop).from,
    reveal: (at) => {
      const position = Math.min(Math.max(at, 0), view.state.doc.length);
      view.dispatch({ selection: { anchor: position }, effects: EditorView.scrollIntoView(position, { y: "start", yMargin: 24 }) });
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
      view.destroy();
    },
  };
}

// Two versions side by side in one read-only view: `to` as text, with
// what `from` had in its place drawn above each change.
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
