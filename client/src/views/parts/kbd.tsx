// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How a key is drawn, in the two places a key is ever drawn: beside the
// thing it reaches, and all together on the sheet `?` opens. Both read
// `core/keys`, so a rebind moves the mark on the rail and the row on the
// sheet at once, and neither file spells a key of its own.

import { For, createSignal, onMount } from "solid-js";

import { ACTIONS, LABELS, keymap, marks } from "../../core/keys";
import type { Action } from "../../core/keys";
import { useSay } from "../../ui";

// The chord that reaches one action, as this machine writes it: `⌘` on a
// Mac and `Ctrl` everywhere else, and a prefixed chord as the two
// keystrokes it is.
export function Kbd(props: { readonly action: Action; readonly class?: string }) {
  const keys = keymap();
  return (
    <span class={`inline-flex items-center gap-tight ${props.class ?? ""}`}>
      <For each={marks(keys.chord(props.action), keys.platform)}>
        {(mark) => (
          <kbd class="rounded-control px-tight font-mono text-note leading-none text-text-disabled no-underline">
            {mark}
          </kbd>
        )}
      </For>
    </span>
  );
}

// Every key at once. A panel rather than the rail held open: reading
// twelve chords is not navigating, and the page underneath should not
// have to move to be read over.
export function Cheatsheet(props: { readonly onClose: () => void }) {
  const say = useSay();
  const [panel, setPanel] = createSignal<HTMLDivElement>();
  onMount(() => {
    panel()?.focus();
  });
  return (
    <div
      class="fixed inset-0 z-20 flex items-start justify-center bg-g0/70 pt-section"
      onClick={() => {
        props.onClose();
      }}
    >
      <div
        ref={setPanel}
        class="rise w-full max-w-measure rounded-panel bg-g1 p-pane shadow-composer"
        role="dialog"
        tabindex="-1"
        aria-label={say("keys_title")}
        onClick={(event) => {
          event.stopPropagation();
        }}
      >
        <div class="mb-base flex items-baseline justify-between">
          <h2 class="text-heading font-heading text-text">{say("keys_title")}</h2>
          <button
            type="button"
            class="rounded-control px-snug py-tight text-label text-text-quiet hover:bg-g2 hover:text-text"
            onClick={() => {
              props.onClose();
            }}
          >
            {say("dismiss")}
          </button>
        </div>
        <ul class="flex flex-col">
          <For each={ACTIONS}>
            {(action) => (
              <li class="flex items-center justify-between gap-base py-tight text-label">
                <span class="truncate text-text-quiet">{say(LABELS[action])}</span>
                <Kbd action={action} />
              </li>
            )}
          </For>
        </ul>
        <p class="mt-base text-note text-text-faint">{say("keys_where")}</p>
      </div>
    </div>
  );
}
