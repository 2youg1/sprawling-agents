// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { isolateHistory } from "@codemirror/commands";
import type { EditorView } from "@codemirror/view";
import { createMemo, createSignal, For, Show } from "solid-js";

import { fill, say, type Key, type Lang } from "../../core/lang";
import { applyUndo, planUndo, undoPreview, undoRows } from "./undo_pool";
import { paragraphIndents } from "./writing_indent";

interface PanelProps {
  readonly view: EditorView;
  readonly lang: Lang;
  readonly revision: () => number;
}

export function WorkspaceUndo(props: PanelProps) {
  const [picked, setPicked] = createSignal<ReadonlySet<number>>(new Set());
  const [notice, setNotice] = createSignal("");
  const word = (key: Key) => say(props.lang, key);
  const rows = createMemo(() => {
    props.revision();
    return undoRows(props.view.state);
  });
  const plan = createMemo(() => {
    props.revision();
    return planUndo(props.view.state, picked());
  });
  const preview = () => {
    const held = plan();
    return held.kind === "ready" ? undoPreview(held) : null;
  };
  const label = (line: number, text: string) =>
    fill(word("writing_undo_change"), { line: String(line), text });

  function pick(id: number, checked: boolean): void {
    setNotice("");
    const next = new Set(picked());
    if (checked) next.add(id);
    else next.delete(id);
    setPicked(next);
  }

  function apply(): void {
    const count = picked().size;
    if (applyUndo(props.view, plan(), picked())) {
      setPicked(new Set<number>());
      setNotice(fill(word("writing_undo_done"), { count: String(count) }));
    } else {
      setNotice(word("writing_undo_stale"));
    }
  }

  return (
    <div class="workspace-edit-panel">
      <div class="workspace-edit-list">
        <For each={rows()}>{(row, index) => (
          <label class="workspace-edit-choice">
            <input type="checkbox" data-control="" data-context-focus={index() === 0 ? "" : undefined}
              checked={picked().has(row.id)} onChange={(event) => pick(row.id, event.currentTarget.checked)} />
            <span>{label(row.line, row.snippet)}</span>
          </label>
        )}</For>
      </div>
      <output role="status">{notice() || (() => {
        const held = plan();
        if (held.kind === "conflict")
          return fill(word("writing_undo_conflict"), { edit: label(held.later.line, held.later.snippet) });
        if (rows().length === 0) return word("writing_undo_empty");
        return held.kind === "empty" ? word("writing_undo_pick") : "";
      })()}</output>
      <Show when={preview()}>{(difference) => (
        <div class="workspace-edit-list workspace-edit-diff">
          <For each={difference().spans}>{(span) => (
            <div>
              <span class="text-text-quiet">{span.line}</span>
              <Show when={span.before !== ""}><del>{span.before}</del></Show>
              <Show when={span.after !== ""}><ins>{span.after}</ins></Show>
            </div>
          )}</For>
          <Show when={difference().spans.length < difference().count}>
            {fill(word("writing_preview_limit"), { count: String(difference().count) })}
          </Show>
        </div>
      )}</Show>
      <div class="workspace-edit-actions justify-end">
        <button type="button" data-control="surface" disabled={plan().kind !== "ready" || props.view.state.readOnly}
          onClick={apply}>{word("writing_undo_apply")}</button>
      </div>
    </div>
  );
}

export function WorkspaceIndent(props: PanelProps & { readonly done: () => void }) {
  const [notice, setNotice] = createSignal("");
  const word = (key: Key) => say(props.lang, key);
  const preview = createMemo(() => {
    props.revision();
    const state = props.view.state;
    return { doc: state.doc, changes: state.changes(paragraphIndents(state)) };
  });
  const lines = createMemo(() => {
    const { doc, changes } = preview();
    const shown: string[] = [];
    let count = 0;
    changes.iterChanges((from) => {
      count += 1;
      if (shown.length < 32) {
        const line = doc.lineAt(from);
        shown.push(fill(word("writing_undo_change"), { line: String(line.number), text: line.text.slice(0, 120) }));
      }
    });
    return { shown, count };
  });
  function apply(): void {
    const pick = preview();
    if (props.view.state.doc !== pick.doc || props.view.compositionStarted) {
      setNotice(word("writing_indent_changed"));
      return;
    }
    if (props.view.state.readOnly || pick.changes.empty) return;
    props.view.dispatch({
      changes: pick.changes, annotations: isolateHistory.of("full"), userEvent: "input.indent",
    });
    props.done();
  }
  return (
    <div class="workspace-edit-panel">
      <output role="status">{notice() || (lines().count === 0
        ? word("writing_indent_empty")
        : fill(word("writing_indent_count"), { count: String(lines().count) }))}</output>
      <div class="workspace-edit-list">
        <For each={lines().shown}>{(line) => <div class="workspace-edit-choice">{line}</div>}</For>
      </div>
      <div class="workspace-edit-actions justify-end">
        <button type="button" data-control="surface" disabled={preview().changes.empty || props.view.state.readOnly}
          onClick={apply}>{word("writing_indent_apply")}</button>
      </div>
    </div>
  );
}
