// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Show } from "solid-js";
import { say, type Lang } from "../../core/lang";
import Glyph from "../parts/glyph.solid";
import type { ContextSelection } from "./editing";
import type { MarkdownMode } from "./markdown_editing";
import { writeReceiptWord, type WriteReceipt } from "./workspace_receipt";

interface Props {
  readonly version: string;
  readonly lang: Lang;
  readonly editable: boolean;
  readonly markdown: boolean;
  readonly mode: MarkdownMode;
  readonly onMode: (mode: MarkdownMode) => void;
  readonly onAttach?: (() => void) | undefined;
  readonly previewable: boolean;
  readonly previewing: boolean;
  readonly onPreview: () => void;
  readonly typewriter: boolean;
  readonly onTypewriter: (on: boolean) => void;
  readonly dirty: boolean;
  readonly changed: boolean;
  readonly receipt: WriteReceipt;
  readonly onSave: () => void;
}

function controlState(receipt: WriteReceipt) {
  switch (receipt.kind) {
    case "saving":
      return "pending";
    case "saved":
      return "success";
    case "conflict":
    case "refused":
      return "refused";
    case "pending":
      return "unknown";
    case "clean":
      return undefined;
  }
}

export function selectionMenuStyle(menu: ContextSelection | null) {
  if (menu === null) return undefined;
  return {
    left: `${String(Math.max(12, Math.min(menu.x, globalThis.innerWidth - 212)))}px`,
    top: `${String(Math.max(12, Math.min(menu.y, globalThis.innerHeight - 64)))}px`,
  };
}

export function WorkspaceDocumentTools(props: Props) {
  const saving = () => props.receipt.kind === "saving";
  return (
    <header class="workspace-document-tools">
      <span class="workspace-document-version" title={props.version}>
        {props.version.slice(0, 7)}
      </span>
      <Show when={props.editable}>
        <Show when={props.markdown}>
          <button
            type="button"
            data-control="surface"
            aria-pressed={props.mode === "source" ? "true" : "false"}
            onClick={() => props.onMode("source")}
          >
            <Glyph name="terminal" size="sm" />
            {say(props.lang, "refrain_source")}
          </button>
          <button
            type="button"
            data-control="surface"
            aria-pressed={props.mode === "live" ? "true" : "false"}
            onClick={() => props.onMode("live")}
          >
            <Glyph name="record" size="sm" />
            {say(props.lang, "refrain_live")}
          </button>
          <Show when={props.onAttach}>
            {(activate) => (
              <button type="button" data-control="surface" onClick={() => activate()()}>
                <Glyph name="pin" size="sm" />
                {say(props.lang, "refrain_attach_selection")}
              </button>
            )}
          </Show>
        </Show>
        <Show when={props.previewable}>
          <button
            type="button"
            data-control="surface"
            aria-pressed={props.previewing ? "true" : "false"}
            onClick={props.onPreview}
          >
            <Glyph name="reveal" size="sm" />
            {say(props.lang, "refrain_preview")}
          </button>
        </Show>
        <label>
          <input
            type="checkbox"
            data-control=""
            checked={props.typewriter}
            onChange={(event) => props.onTypewriter(event.currentTarget.checked)}
          />
          {say(props.lang, "refrain_typewriter")}
        </label>
        <button
          type="button"
          data-control="surface"
          data-control-state={controlState(props.receipt)}
          aria-busy={saving() ? "true" : undefined}
          disabled={!props.dirty || saving() || props.changed}
          onClick={props.onSave}
        >
          <Glyph name={saving() ? "pulse" : "check"} size="sm" />
          {say(props.lang, "refrain_save")}
        </button>
      </Show>
      <span role="status">{writeReceiptWord(props.receipt, props.dirty, props.lang)}</span>
    </header>
  );
}
