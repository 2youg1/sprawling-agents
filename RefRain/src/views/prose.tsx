// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, onCleanup, Show } from "solid-js";
import { storeValue } from "../core/solid_store";
import Laid from "./editor/laid.solid";
import { laidReply } from "./reply";
import { copiedSource } from "./talk/copying";

export interface ProseProps {
  readonly text: string;
}

export default function Prose(props: ProseProps) {
  const laid = laidReply(() => props.text, "settled");
  const drawn = storeValue(laid.state);
  createEffect(
    () => props.text,
    (text) => {
      laid.update(text);
    },
  );
  onCleanup(() => {
    laid.close();
  });
  const rest = createMemo(() => props.text.slice(drawn().reached));

  function copy(event: ClipboardEvent & { readonly currentTarget: HTMLElement }): void {
    const area = event.currentTarget;
    const chosen = document.getSelection();
    if (chosen?.rangeCount !== 1 || !area.contains(chosen.getRangeAt(0).commonAncestorContainer))
      return;
    const source = copiedSource(chosen.toString(), area.textContent, props.text);
    if (source === null || event.clipboardData === null) return;
    event.clipboardData.setData("text/plain", source);
    event.preventDefault();
  }

  return (
    <div onCopy={copy}>
      <Laid blocks={drawn().blocks} />
      <Show when={rest()}>
        {(words) => (
          <p class="my-snug whitespace-pre-wrap break-words font-read leading-read">{words()}</p>
        )}
      </Show>
    </div>
  );
}
