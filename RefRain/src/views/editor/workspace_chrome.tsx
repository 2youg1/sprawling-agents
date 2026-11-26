// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createSignal, createUniqueId, onCleanup, Show } from "solid-js";

import { fill, say, type Lang } from "../../core/lang";
import { presser, REST, type Copied } from "../parts/copy";
import Tip from "../parts/control_tip";
import Glyph from "../parts/glyph.solid";
import type { MarkdownMode } from "./markdown_editing";
import { writeReceiptWord, type WriteReceipt } from "./workspace_receipt";

export function MarkdownFileHead(props: {
  readonly name: string;
  readonly lang: Lang;
  readonly mode: MarkdownMode;
  readonly editable: boolean;
  readonly dirty: boolean;
  readonly receipt: WriteReceipt;
  readonly onMode: (mode: MarkdownMode) => void;
  readonly onClose?: (() => void) | undefined;
}) {
  const warning = () => ["conflict", "refused", "pending"].includes(props.receipt.kind);
  const waiting = () => props.dirty || props.receipt.kind === "saving";
  const label = () =>
    writeReceiptWord(props.receipt, props.dirty, props.lang) ||
    say(props.lang, "refrain_receipt_saved");
  return (
    <header class="refrain-file-header workspace-markdown-header">
      <div class="workspace-file-name">
        <h2 title={props.name}>{props.name}</h2>
        <span role="status" class="workspace-file-status" data-draft={waiting() ? "" : undefined}>
          <Glyph name={warning() ? "cross" : waiting() ? "ring" : "check"} size="sm" />
          <span class={warning() ? "" : "sr-only"}>{label()}</span>
        </span>
      </div>
      <Show when={props.editable}>
        <label class="workspace-live-switch">
          <input
            type="checkbox"
            role="switch"
            data-control=""
            checked={props.mode === "live"}
            onChange={(event) => props.onMode(event.currentTarget.checked ? "live" : "source")}
          />
          <span class="workspace-live-track" aria-hidden="true" />
          <span>{say(props.lang, "refrain_live")}</span>
        </label>
      </Show>
      <Show when={props.onClose} keyed>
        {(close) => (
          <button type="button" data-control="surface" class="refrain-nav-button" onClick={close}>
            {say(props.lang, "shell_close_file")}
          </button>
        )}
      </Show>
    </header>
  );
}

export function MarkdownReadout(props: {
  readonly lang: Lang;
  readonly draft: number;
  readonly line: number;
  readonly version: string;
}) {
  const [copied, setCopied] = createSignal<Copied>(REST);
  const hint = createUniqueId();
  let timer: ReturnType<typeof setTimeout> | undefined;
  let alive = true;
  onCleanup(() => {
    alive = false;
    clearTimeout(timer);
  });
  const copy = presser({
    board: () => navigator.clipboard,
    later: (run, ms) => {
      if (!alive) return () => undefined;
      const pending = setTimeout(run, ms);
      timer = pending;
      return () => clearTimeout(pending);
    },
    show: (status) => {
      if (alive) setCopied(status);
    },
  });
  const label = () => say(props.lang, "writing_copy_digest");
  const digest = () => `${say(props.lang, "writing_blake3")} ${props.version.slice(0, 9)}…`;
  const receipt = () =>
    copied().kind === "copied"
      ? say(props.lang, "writing_copied_digest")
      : copied().kind === "refused"
        ? say(props.lang, "writing_copy_failed")
        : "";
  const format = (value: number) => value.toLocaleString(props.lang);
  return (
    <div class="workspace-writing-readout">
      <p class="workspace-writing-bytes">
        <span>{fill(say(props.lang, "writing_global_bytes"), { size: format(props.draft) })}</span>
        <span>{fill(say(props.lang, "writing_line_bytes"), { size: format(props.line) })}</span>
      </p>
      <div class="workspace-writing-digest">
        <Tip text={receipt() || label()} id={hint}>
          <button
            type="button"
            data-control="surface"
            data-copy-receipt={copied().kind}
            aria-label={label()}
            aria-describedby={hint}
            onClick={() => copy(props.version)}
          >
            <span>{digest()}</span>
            <Glyph
              name={
                copied().kind === "copied"
                  ? "check"
                  : copied().kind === "refused"
                    ? "cross"
                    : "copy"
              }
              size="sm"
            />
          </button>
        </Tip>
        <output class="sr-only" role="status">
          {receipt()}
        </output>
      </div>
    </div>
  );
}
