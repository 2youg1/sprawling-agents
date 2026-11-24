// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Show } from "solid-js";
import { say, type Lang } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { GalleryCase } from "./case";
import { EARLIER_SEGMENT, ROUND, type Utterance } from "./served";

const LINES: readonly Utterance[] = [...EARLIER_SEGMENT, ...ROUND];
const SETTLED =
  "The mayor kept three things: the transcript of the first day, and the screenshot of the settings page under forced colours, filed where the registry lists them";
const EDGE = " newest first";

function UtteranceLook(props: { readonly line: Utterance; readonly lang: Lang }) {
  return (
    <Show
      when={props.line.speaker === "person"}
      fallback={
        <div class="my-base text-body">
          <div class="mb-tight text-note text-text-faint">{say(props.lang, "talk_resident")}</div>
          <div class="whitespace-pre-wrap leading-read">{props.line.text}</div>
        </div>
      }
    >
      <div class="my-base flex flex-col items-end">
        <div class="max-w-[83%] rounded-panel bg-speech px-pane py-base text-body leading-read whitespace-pre-wrap">
          {props.line.text}
        </div>
        <div class="mt-tight text-note text-text-faint">{say(props.lang, "talk_you")}</div>
      </div>
    </Show>
  );
}

function AnchoredThread(props: { readonly lang: Lang; readonly follow?: boolean }) {
  return (
    <div class="h-output overflow-y-auto">
      <div
        class={["flex min-h-full w-full flex-col px-pane pb-wide", { "justify-end": props.follow }]}
      >
        <For each={LINES}>{(line) => <UtteranceLook line={line} lang={props.lang} />}</For>
        <div class="my-base text-body">
          <div class="mb-tight text-note text-text-faint">{say(props.lang, "talk_resident")}</div>
          <div class="whitespace-pre-wrap leading-read">
            {SETTLED}
            <span class="text-text-faint">{EDGE}</span>
            <span class="blink ml-tight inline-block size-[6px] bg-accent align-baseline" />
          </div>
        </div>
      </div>
    </div>
  );
}

export default function AnchoredCases() {
  const lang = storeValue(ui().lang);
  return (
    <>
      <GalleryCase label="thread · at the foot, the growing edge followed">
        <AnchoredThread lang={lang()} follow />
      </GalleryCase>
      <GalleryCase label="thread · reading back, the place held while words arrive">
        <AnchoredThread lang={lang()} />
      </GalleryCase>
    </>
  );
}
