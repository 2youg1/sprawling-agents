// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createUniqueId, Show, untrack } from "solid-js";
import { storeValue } from "../../core/solid_store";
import { dictation } from "../../core/speaking";
import { ui } from "../../ui";
import ControlTip from "../parts/control_tip";
import Glyph from "../parts/glyph.solid";
import { lookOf, type RecordLook } from "./record";
import { speakAsked } from "./speak_asked";
import "./coin_view.css";

export function RecordKey(props: RecordLook) {
  const id = createUniqueId();
  const state = () =>
    props.refused !== undefined ? "refused" : props.hearing === "hearing" ? "pending" : undefined;
  return (
    <>
      <ControlTip id={id} text={props.label}>
        <button
          type="button"
          data-control=""
          class="talk-microphone transition-[background-color,color,scale] ease-leave hover:ease-arrive aria-[disabled=false]:active:scale-(--scale-press) aria-[disabled=false]:active:ease-arrive still:transition-none still:aria-[disabled=false]:active:scale-100"
          data-hearing={props.hearing}
          aria-label={props.label}
          aria-disabled={props.wire["aria-disabled"] ? "true" : "false"}
          data-control-state={state()}
          onClick={() => props.wire.onclick()}
        >
          <Glyph name="mic" size="sm" />
        </button>
      </ControlTip>
      <Show when={props.refused}>
        {(reason) => <span class="text-note text-alert">{reason()}</span>}
      </Show>
    </>
  );
}

export default function Record(props: { readonly onWords: (words: string) => void }) {
  const u = ui();
  const lang = storeValue(u.lang);
  const heard = dictation(u.origin, u.credential, (words) => {
    props.onWords(words);
  });
  const taking = storeValue(heard.taking);
  const transcribing = storeValue(heard.hearing);
  const refused = storeValue(heard.refused);
  const asked = storeValue(speakAsked);
  function press(): void {
    if (!transcribing()) heard.speak();
  }
  let last = untrack(asked);
  createEffect(asked, (now) => {
    if (now === last) return;
    last = now;
    press();
  });
  const look = createMemo(() =>
    lookOf({ taking: taking(), hearing: transcribing(), refused: refused() }, lang(), press),
  );
  return <RecordKey {...look()} />;
}
