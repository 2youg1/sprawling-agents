// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, onCleanup } from "solid-js";
import { storeValue } from "../../core/solid_store";
import Laid from "../editor/laid.solid";
import { laidReply } from "../reply";
import { faded } from "./saying";
import "./marks.css";

export interface SayingProps {
  readonly text: string;
  readonly who: string;
}

export default function Saying(props: SayingProps) {
  const laid = laidReply(() => props.text, "streaming");
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
  const tail = createMemo(() => faded(props.text.slice(drawn().reached)));
  return (
    <div class="my-pane">
      <div class="text-body">
        <div class="mb-tight text-note text-text-faint">{props.who}</div>
        <Laid blocks={drawn().blocks} />
        <div class="whitespace-pre-wrap break-words font-read leading-read">
          {tail().settled}
          <span class="text-text-faint">{tail().edge}</span>
          <span class="talk-caret blink bg-accent" />
        </div>
      </div>
    </div>
  );
}
