// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, flush, onCleanup, Show, untrack } from "solid-js";
import { storeValue } from "../../../core/solid_store";
import { ui } from "../../../ui";
import type { EndpointsAnswer } from "../../../wire";
import Button from "../../parts/button.solid";
import ModelTable from "../model_table.solid";
import { editorState } from "./editor_state";
import type { FlightState } from "./flight";
import Generation from "./generation.solid";
import {
  awaiting,
  choose,
  freshModelEditor,
  leave,
  refuse,
  settleAnswer,
  settleProbe,
  summaryOf,
  type Hands,
} from "./model_editor";
import { lookOf } from "./model_editor_look";

export interface ModelEditorProps {
  readonly answer: EndpointsAnswer;
  readonly name: string;
  readonly flight: FlightState;
  readonly open?: boolean | undefined;
}

export default function ModelEditor(props: ModelEditorProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const editor = editorState(
    untrack(() => freshModelEditor(props.answer, props.name, props.flight)),
  );
  const [tableRevision, setTableRevision] = createSignal(0);
  const [expanded, setExpanded] = createSignal(untrack(() => props.open ?? false));
  let summary: HTMLElement | undefined;
  const hands: Hands = {
    answer: () => props.answer,
    probed: () => belief().probed,
    send: (command) => u.send(command),
  };
  const endpoint = createMemo(() => summaryOf(props.answer, props.name));
  const about = createMemo(() => {
    const held = endpoint();
    return held === undefined
      ? null
      : { endpoint: held.name, baseUrl: held.base_url, face: held.dialect };
  });
  const look = createMemo(() => lookOf(editor, hands, lang()));
  const refreshTable = () => setTableRevision((revision) => revision + 1);
  createEffect(
    () => belief().probed,
    (result) => {
      settleProbe(editor, result);
      refreshTable();
    },
  );
  createEffect(
    () => props.answer,
    (answer) => {
      settleAnswer(editor, answer);
      refreshTable();
    },
  );
  createEffect(
    () => belief().refusal,
    (error) => {
      if (error === null || !awaiting(editor)) return;
      refuse(editor, error);
      u.conn.dismissRefusal();
    },
  );
  createEffect(
    () => editor.focusSummary,
    (wanted) => {
      if (!wanted) return;
      editor.focusSummary = false;
      queueMicrotask(() => {
        flush();
        summary?.focus();
      });
    },
  );
  onCleanup(() => leave(editor));

  return (
    <>
      <details
        class="mt-snug"
        open={expanded()}
        onToggle={(event) => setExpanded(event.currentTarget.open)}
      >
        <summary
          data-control=""
          class="w-fit cursor-pointer rounded-control px-tight text-text-quiet transition-[background-color] ease-leave hover:bg-raised hover:ease-arrive"
          ref={(node) => {
            summary = node;
          }}
        >
          {look().summary}
        </summary>
        <div class="flex flex-col gap-snug pt-snug">
          <div class="flex flex-wrap items-center gap-snug">
            <Button {...look().refresh} />
          </div>
          <Show when={look().removal}>
            {(removal) => <p class="text-note text-text-quiet">{removal()}</p>}
          </Show>
          <ModelTable
            served={editor.served}
            about={about()}
            state={editor.table}
            revision={tableRevision()}
            onChosen={(rows) => choose(editor, rows)}
          />
        </div>
      </details>
      <div class="mt-snug flex flex-col gap-snug border-t border-edge pt-snug">
        <h4 class="text-label font-label text-text-quiet">{look().generationHeading}</h4>
        <Show when={endpoint()}>
          {(held) => (
            <Generation
              models={held().models.filter((model) =>
                editor.chosen.some((row) => row.id === model.id),
              )}
              summary={held()}
              drafts={editor.generation}
              overrideRows={editor.overrides}
              onEdit={() => choose(editor, [...editor.chosen])}
            />
          )}
        </Show>
        <div class="flex flex-wrap items-center gap-snug">
          <Button {...look().save} />
        </div>
        <Show when={look().busy}>
          {(busy) => (
            <p role="status" class="text-note text-text-quiet">
              {busy()}
            </p>
          )}
        </Show>
        <Show when={look().alert}>
          {(alert) => (
            <div
              role="alert"
              class="rounded-card border border-alert/50 bg-chrome px-base py-snug text-note"
            >
              <p class="font-label text-alert">{alert().title}</p>
              <Show when={alert().subject}>
                {(subject) => <p class="mt-tight font-mono wrap-anywhere text-text">{subject()}</p>}
              </Show>
              <Show when={alert().code}>
                {(code) => <p class="mt-tight font-mono text-text-faint">{code()}</p>}
              </Show>
            </div>
          )}
        </Show>
        <Show when={look().status}>
          {(status) => (
            <p role="status" class="text-note text-text-quiet">
              {status()}
            </p>
          )}
        </Show>
      </div>
    </>
  );
}
