// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Show } from "solid-js";
import type { DialogLookProps } from "../../../src/views/parts/dialog.look.solid";
import "./dialog.solid.css";

export default function AlternateDialog(props: DialogLookProps) {
  return (
    <dialog
      class="swap-question"
      data-in-flow={props.look.stands === "in-flow" ? "" : undefined}
      aria-labelledby={props.look.sheet["aria-labelledby"] ?? undefined}
      aria-describedby={props.look.sheet["aria-describedby"] ?? undefined}
      onCancel={(event) => props.look.sheet.oncancel?.(event)}
      ref={(node) => {
        if (props.drawn === true) node.open = true;
        props.handle?.(node);
      }}
    >
      <h2 id={props.look.heading.id}>{props.look.heading.text}</h2>
      <Show when={props.look.detail}>{(detail) => <p id={detail().id}>{detail().text}</p>}</Show>
      {props.look.grounds}
      <div class="swap-answers">
        <For each={props.look.answers} keyed={(answer) => answer.tone}>
          {(answer) => (
            <button
              type="button"
              data-control=""
              data-tone={answer().tone}
              onClick={() => answer().onPress()}
            >
              {answer().label}
            </button>
          )}
        </For>
      </div>
    </dialog>
  );
}
