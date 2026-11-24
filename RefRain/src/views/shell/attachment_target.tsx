// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, For, onSettled, Show } from "solid-js";

import { say, type Lang } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import type { Stretch } from "../../core/stretches";
import { ui } from "../../ui";
import type { Seq } from "../../wire";
import type { DocumentAttachment } from "../talk/attachments";
import { titleOf } from "../world/sessions";

interface Props {
  readonly attachment: DocumentAttachment;
  readonly choosing: boolean;
  readonly current?: Seq | undefined;
  readonly lang: Lang;
  readonly targets: readonly Stretch[];
  readonly onUse: (at: Seq | undefined) => void;
  readonly onNew: () => void;
  readonly onCancel: () => void;
}

export function AttachmentTarget(props: Props) {
  const branches = storeValue(ui().branches.held);
  const refusal = createMemo(() => {
    const state = branches();
    return state.kind === "refused" ? state.error.recovery : undefined;
  });
  let dialog: HTMLDialogElement | undefined;
  onSettled(() => {
    const shown = dialog;
    shown?.showModal();
    return () => shown?.close();
  });
  return (
    <dialog
      ref={(node) => {
        dialog = node;
      }}
      class="refrain-target"
      aria-label={say(props.lang, "shell_choose_target")}
      onCancel={(event) => {
        event.preventDefault();
        if (!props.choosing) props.onCancel();
      }}
    >
      <h2>{say(props.lang, "shell_choose_target")}</h2>
      <p>{props.attachment.label}</p>
      <Show when={refusal()}>{(message) => <p role="alert">{message()}</p>}</Show>
      <div class="refrain-files-tools">
        <button
          data-control=""
          class="refrain-nav-button"
          type="button"
          disabled={props.choosing}
          onClick={() => props.onUse(undefined)}
        >
          {say(props.lang, "shell_use_session")}
        </button>
        <button
          data-control=""
          class="refrain-nav-button"
          type="button"
          disabled={props.choosing}
          onClick={() => props.onNew()}
        >
          {say(props.lang, "shell_new_session")}
        </button>
        <button
          data-control=""
          class="refrain-nav-button"
          type="button"
          disabled={props.choosing}
          onClick={() => props.onCancel()}
        >
          {say(props.lang, "shell_cancel")}
        </button>
      </div>
      <For each={props.targets.filter((stretch) => stretch.line.began !== props.current)}>
        {(stretch) => (
          <button
            data-control=""
            class="refrain-nav-button"
            type="button"
            disabled={props.choosing}
            onClick={() => props.onUse(stretch.line.began)}
          >
            {titleOf(stretch)}
          </button>
        )}
      </For>
    </dialog>
  );
}
