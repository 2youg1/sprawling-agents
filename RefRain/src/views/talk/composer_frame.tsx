// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { createEffect, Show } from "solid-js";
import type { ComposerLook } from "./composer";
import { AttachmentDrawer } from "./attachment_drawer";
import { SpeechFeedback } from "./speech_feedback";

type SlotName = "menu" | "band" | "door" | "keys" | "line" | "under";

export interface ComposerFrameProps {
  readonly look: ComposerLook;
  readonly door: boolean;
  readonly slots: Readonly<Partial<Record<SlotName, JSX.Element>>>;
}

export function ComposerFrame(props: ComposerFrameProps) {
  let input: HTMLTextAreaElement | undefined;
  createEffect(
    () => props.look.attachBox,
    (attach) => (input === undefined ? undefined : attach?.(input)),
  );
  return (
    <form
      class={[
        "relative rounded-control transition-colors",
        props.look.over ? "wash ease-arrive" : "ease-leave",
      ]}
      aria-label={props.look.form["aria-label"] ?? undefined}
      onSubmit={(event) => {
        props.look.form.onsubmit?.call(event.currentTarget, event);
      }}
      onDragOver={(event) => {
        props.look.form.ondragover?.call(event.currentTarget, event);
      }}
      onDragLeave={(event) => {
        props.look.form.ondragleave?.call(event.currentTarget, event);
      }}
      onDrop={(event) => {
        props.look.form.ondrop?.call(event.currentTarget, event);
      }}
    >
      {props.slots.menu}
      {props.slots.band}
      <Show when={props.look.attachments}>{(details) => <AttachmentDrawer {...details()} />}</Show>
      <div class="relative pb-snug">
        <div class="flex min-h-key items-end gap-base">
          <div class="relative min-w-0 flex-1">
            <Show when={props.door}>
              <div class="pointer-events-none absolute inset-x-0 top-0 z-(--layer-raised) flex py-snug text-body leading-read text-text-faint">
                {props.slots.door}
              </div>
            </Show>
            <textarea
              class="block max-h-composer min-h-key w-full resize-none overflow-y-auto bg-transparent py-snug text-body leading-read text-text caret-accent outline-hidden field-sizing-content placeholder:text-text-faint"
              ref={(element) => {
                input = element;
              }}
              rows={props.look.box.rows ?? undefined}
              autofocus={props.look.box.autofocus ?? undefined}
              placeholder={props.look.box.placeholder ?? undefined}
              aria-label={props.look.box["aria-label"] ?? undefined}
              aria-activedescendant={props.look.box["aria-activedescendant"] ?? undefined}
              aria-controls={props.look.box["aria-controls"] ?? undefined}
              onKeyDown={(event) => {
                props.look.box.onkeydown?.call(event.currentTarget, event);
              }}
              onInput={(event) => {
                props.look.box.oninput?.call(event.currentTarget, event);
              }}
              onFocus={(event) => {
                props.look.box.onfocus?.call(event.currentTarget, event);
              }}
              onBlur={(event) => {
                props.look.box.onblur?.call(event.currentTarget, event);
              }}
            />
          </div>
          {props.slots.keys}
        </div>
        {props.slots.line}
      </div>
      <Show when={props.look.speech}>{(speech) => <SpeechFeedback {...speech()} />}</Show>
      {props.slots.under}
    </form>
  );
}
