// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { For, Show } from "solid-js";

import Button from "./button.solid";
import type { DialogLook } from "./dialog";
import { FACE, STANDS } from "./sheet_paint";

export interface DialogLookProps {
  readonly look: DialogLook<JSX.Element>;
  readonly handle?: ((node: HTMLDialogElement) => void) | undefined;
  readonly drawn?: boolean | undefined;
}

export default function DialogLookView(props: DialogLookProps) {
  const look = () => props.look;
  return (
    <dialog
      ref={(node) => {
        if (props.drawn === true) node.open = true;
        props.handle?.(node);
      }}
      aria-labelledby={look().heading.id}
      aria-describedby={look().detail?.id}
      onCancel={(event) => look().sheet.oncancel?.(event)}
      class={[FACE, STANDS[look().stands]]}
    >
      <h2 id={look().heading.id} class="text-heading text-text">
        {look().heading.text}
      </h2>
      <Show when={look().detail}>
        {(detail) => (
          <p id={detail().id} class="text-note text-text-quiet">
            {detail().text}
          </p>
        )}
      </Show>
      {look().grounds}
      <div class="flex items-center justify-end gap-snug">
        <For each={look().answers} keyed={(answer) => answer.tone}>
          {(answer) => (
            <Button
              label={answer().label}
              tone={answer().tone}
              onPress={() => answer().onPress()}
            />
          )}
        </For>
      </div>
    </dialog>
  );
}
