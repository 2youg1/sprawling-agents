// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Show } from "solid-js";
import { fill, say, type Lang } from "../../core/lang";
import Glyph from "../parts/glyph.solid";
import type { ContextTool } from "../parts/context_tools";
import { CONTEXT_FORMATS, markdownShortcut, type MarkdownFormat } from "./markdown_editing";
import { writeReceiptWord, type WriteReceipt } from "./workspace_receipt";

interface Props {
  readonly lang: Lang;
  readonly editable: boolean;
  readonly previewable: boolean;
  readonly previewing: boolean;
  readonly onPreview: () => void;
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

export function workspaceSelectionTools(
  lang: Lang,
  format: ((wanted: MarkdownFormat) => void) | undefined,
  attach: (() => void) | undefined,
): readonly ContextTool[] {
  const tools: ContextTool[] =
    format === undefined
      ? []
      : CONTEXT_FORMATS.map((wanted) => ({
          id: wanted.id,
          label: say(lang, wanted.label),
          ...(wanted.id === "quote" ? {} : { shortcut: markdownShortcut(wanted) }),
          run: () => format(wanted),
        }));
  if (attach !== undefined)
    tools.push({
      id: "attach",
      glyph: "pin",
      label: say(lang, "refrain_attach_selection"),
      run: attach,
    });
  return tools;
}

export function WorkspaceDocumentTools(props: Props) {
  const saving = () => props.receipt.kind === "saving";
  return (
    <header class="workspace-document-tools">
      <Show when={props.editable}>
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

export function WorkspaceDocumentFeedback(props: {
  readonly lang: Lang;
  readonly changed: boolean;
  readonly staleDraft: string | undefined;
  readonly draftUnkept: boolean;
  readonly receipt: WriteReceipt;
  readonly attachRefused: boolean;
  readonly draft: () => string | undefined;
}) {
  return (
    <>
      <Show when={props.changed}>
        <p role="status">{say(props.lang, "refrain_external_changed")}</p>
      </Show>
      <Show when={props.staleDraft !== undefined}>
        <div class="workspace-document-error" role="status">
          <p>{say(props.lang, "refrain_previous_draft")}</p>
          <button
            type="button"
            data-control="surface"
            onClick={() => {
              const stored = props.staleDraft;
              if (stored !== undefined) void navigator.clipboard.writeText(stored);
            }}
          >
            <Glyph name="copy" size="sm" />
            {say(props.lang, "refrain_copy_draft")}
          </button>
        </div>
      </Show>
      <Show when={props.draftUnkept}>
        <p role="status">{say(props.lang, "refrain_registered_draft_unkept")}</p>
      </Show>
      <Show when={props.receipt.detail !== undefined}>
        <details class="workspace-document-error">
          <summary>{props.receipt.detail}</summary>
          <Show when={props.receipt.diagnostic !== undefined}>
            <pre>{props.receipt.diagnostic}</pre>
            <button
              type="button"
              data-control="surface"
              onClick={() => {
                const diagnostic = props.receipt.diagnostic;
                if (diagnostic !== undefined) void navigator.clipboard.writeText(diagnostic);
              }}
            >
              <Glyph name="copy" size="sm" />
              {say(props.lang, "refrain_copy_diagnostic")}
            </button>
          </Show>
          <button
            type="button"
            data-control="surface"
            onClick={() => {
              const text = props.draft();
              if (text !== undefined) void navigator.clipboard.writeText(text);
            }}
          >
            <Glyph name="copy" size="sm" />
            {say(props.lang, "refrain_copy_draft")}
          </button>
        </details>
      </Show>
      <Show when={props.attachRefused}>
        <p role="status">{say(props.lang, "refrain_attach_unsaved")}</p>
      </Show>
    </>
  );
}

export function WorkspaceDocumentStatus(props: {
  readonly lang: Lang;
  readonly version: string;
  readonly bytes: { readonly total: number; readonly line: number } | undefined;
}) {
  const count = (value: number): string => value.toLocaleString(props.lang);
  return (
    <footer class="workspace-document-status">
      <Show when={props.bytes}>
        {(bytes) => (
          <span>
            {fill(say(props.lang, "refrain_byte_count"), {
              total: count(bytes().total),
              line: count(bytes().line),
            })}
          </span>
        )}
      </Show>
      <span class="workspace-document-version" title={props.version}>
        {fill(say(props.lang, "refrain_blake3"), { hash: props.version.slice(0, 7) })}
      </span>
    </footer>
  );
}
