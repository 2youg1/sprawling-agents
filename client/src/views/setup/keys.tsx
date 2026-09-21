// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Where a key is changed. One row per action: what it does, the chord
// that reaches it now, a button that listens for the next chord, and -
// when two actions claim one chord - which other action it collides
// with. Nothing is refused here; a collision is shown and left to the
// person, because the chord they just pressed is the one they meant.
//
// This section is mounted by the settings page, which owns its own
// column and its own section chrome; the heading below is the same one
// that page draws so the section reads the same wherever it is placed.

import { For, Show, createSignal } from "solid-js";

import { ACTIONS, LABELS, keymap, spell } from "../../core/keys";
import type { Action, Chord } from "../../core/keys";
import { useSay } from "../../ui";
import { Kbd } from "../parts/kbd";
import { Tip } from "../parts/tip";

// Keys that are only ever half of a chord.
const MODIFIERS = ["Control", "Meta", "Shift", "Alt", "AltGraph", "CapsLock"];

export function KeysSection() {
  const say = useSay();
  const keys = keymap();
  // The action listening for its new chord.
  const [recording, setRecording] = createSignal<Action | null>(null);

  const stop = () => {
    setRecording(null);
  };
  const taken = (action: Action): readonly Action[] => {
    const spelled = spell(keys.chord(action));
    const clash = keys.conflicts().find((held) => held.spelled === spelled);
    return clash === undefined ? [] : clash.actions.filter((other) => other !== action);
  };
  const capture = (action: Action, event: KeyboardEvent) => {
    if (MODIFIERS.includes(event.key)) {
      return;
    }
    event.preventDefault();
    if (event.key === "Escape") {
      stop();
      return;
    }
    // Shift is part of a chord only beside the accelerator: without
    // one, the browser already hands back the character the layout
    // produced, and judging shift as well would put `?` out of reach
    // on a keyboard that needs shift to type it.
    const accel = event.ctrlKey || event.metaKey;
    const chord: Chord = { accel, shift: accel && event.shiftKey, key: event.key };
    keys.bind(action, chord);
    stop();
  };

  return (
    <section class="border-t border-g1 py-wide">
      <h2 class="mb-base text-heading font-heading">{say("keys_title")}</h2>
      <ul class="flex flex-col">
        <For each={ACTIONS}>
          {(action) => (
            <li class="flex items-center gap-base border-b border-g1 py-snug text-label last:border-b-0">
              <span class="min-w-0 flex-1 truncate text-text-quiet">{say(LABELS[action])}</span>
              <Show when={taken(action).length > 0}>
                <span class="truncate text-note text-alert">
                  {say("keys_conflict", { name: say(LABELS[taken(action)[0] ?? action]) })}
                </span>
              </Show>
              <Tip text={say("keys_change")}>
                {(hint) => (
                  <button
                    type="button"
                    class="rounded-control border border-g2 px-snug py-tight hover:bg-g2 aria-pressed:border-accent"
                    aria-describedby={hint}
                    aria-pressed={recording() === action}
                    onClick={() => {
                      setRecording(action);
                    }}
                    onBlur={stop}
                    onKeyDown={(event) => {
                      if (recording() === action) {
                        capture(action, event);
                      }
                    }}
                  >
                    <Show when={recording() === action} fallback={<Kbd action={action} />}>
                      <span class="font-mono text-note text-text-faint">
                        {say("keys_press")}
                      </span>
                    </Show>
                  </button>
                )}
              </Tip>
              <button
                type="button"
                class="rounded-control px-snug py-tight text-note text-text-faint hover:bg-g2 hover:text-text disabled:invisible"
                disabled={!keys.changed(action)}
                onClick={() => {
                  keys.reset(action);
                }}
              >
                {say("keys_reset")}
              </button>
            </li>
          )}
        </For>
      </ul>
      <button
        type="button"
        class="mt-base rounded-control px-snug py-tight text-label text-text-quiet hover:bg-g2 hover:text-text"
        onClick={() => {
          keys.resetAll();
        }}
      >
        {say("keys_reset_all")}
      </button>
    </section>
  );
}
