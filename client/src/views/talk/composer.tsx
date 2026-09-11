// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The box a person writes in. One textarea, Enter sends, and beside it
// only the two things that belong to a message: how hard the model
// thinks, and - while a run is going - the way to stop it. Nothing
// here decides where a message goes; the page does.

import { Show, createSignal, untrack } from "solid-js";

import type { Sending } from "../../core/belief";
import type { Key } from "../../core/lang";
import { EFFORTS } from "../../core/prefs";
import { useSay, useUi } from "../../ui";

// What the send control is spelled, for each of the three places a
// message can land. `queued` is the one a streaming page would otherwise
// hide: the run is inside a tool call, and the words wait for a boundary.
const SPELLING: Record<Sending, Key> = {
  dispatch: "talk_send",
  steer: "talk_steer",
  queued: "talk_steer_queued",
};

export interface ComposerProps {
  readonly placeholder: string;
  readonly sending: Sending;
  // Where the unsent words are kept, when they are kept at all: the
  // room or the run this box speaks to.
  readonly draft?: string | undefined;
  // Both answer whether the frame went out, so the box can keep the
  // words when it did not.
  readonly onSend: (text: string) => boolean;
  readonly onStop: () => boolean;
}

export function Composer(props: ComposerProps) {
  const ui = useUi();
  const say = useSay();
  const [text, setText] = createSignal(untrack(() => (props.draft === undefined ? "" : ui.prefs.draft(props.draft))));
  const [kept, setKept] = createSignal(false);
  const [box, setBox] = createSignal<HTMLTextAreaElement>();
  const keep = (words: string) => {
    if (props.draft !== undefined) ui.prefs.setDraft(props.draft, words);
  };

  const grow = () => {
    const area = box();
    if (area === undefined) return;
    area.style.height = "auto";
    area.style.height = `${String(Math.min(area.scrollHeight, 240))}px`;
  };
  const submit = () => {
    const words = text().trim();
    if (words === "") return;
    if (props.onSend(words)) {
      setText("");
      keep("");
      setKept(false);
      requestAnimationFrame(grow);
    } else {
      setKept(true);
    }
  };
  const cycleEffort = () => {
    const at = EFFORTS.indexOf(ui.prefs.effort());
    ui.prefs.setEffort(EFFORTS[(at + 1) % EFFORTS.length] ?? "medium");
  };

  return (
    <form
      class="rounded-panel bg-g1 p-base shadow-composer"
      onSubmit={(event) => {
        event.preventDefault();
        submit();
      }}
      aria-label={say("region_composer")}
    >
      <textarea
        ref={(area) => {
          setBox(area);
          // A draft restored on mount is taller than one row.
          requestAnimationFrame(grow);
        }}
        class="block w-full resize-none bg-transparent text-body leading-relaxed text-text outline-none placeholder:text-text-disabled"
        rows={1}
        placeholder={props.placeholder}
        value={text()}
        autofocus
        onInput={(event) => {
          setText(event.currentTarget.value);
          keep(event.currentTarget.value);
          grow();
        }}
        onKeyDown={(event) => {
          if (event.key === "Enter" && !event.shiftKey && !event.isComposing) {
            event.preventDefault();
            submit();
          }
        }}
      />
      <div class="mt-snug flex items-center justify-between text-note text-text-faint">
        <div class="flex items-center gap-base">
          <button
            type="button"
            class="rounded-pill bg-g2 px-base py-tight text-note text-text-quiet hover:bg-g3"
            onClick={cycleEffort}
            title={say("talk_effort")}
          >
            {say("talk_effort")} · {say(`effort_${ui.prefs.effort()}`)}
          </button>
          <Show when={kept()}>
            <span class="text-alert">{say("talk_not_live")}</span>
          </Show>
        </div>
        <div class="flex items-center gap-base">
          <Show when={props.sending !== "dispatch"}>
            <button
              type="button"
              class="rounded-control px-snug py-tight text-label text-text-quiet hover:bg-g2 hover:text-alert"
              onClick={() => props.onStop()}
            >
              {say("talk_stop")}
            </button>
          </Show>
          <button
            type="submit"
            class="rounded-control bg-accent px-base py-tight text-label text-g0 hover:bg-accent-hover disabled:bg-g3 disabled:text-text-disabled"
            disabled={text().trim() === ""}
          >
            {say(SPELLING[props.sending])}
          </button>
        </div>
      </div>
    </form>
  );
}
