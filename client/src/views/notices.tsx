// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The bell: what this session was refused, after the corner has let go
// of it.
//
// **A refusal a person waved away is still the answer to what they
// asked.** Before this, dismissing one destroyed the only copy: the
// three parts the city wrote were on screen once, for as long as
// somebody left them there. The corner still carries the newest one,
// because that is the one they are waiting for; everything that reached
// the corner is here afterwards, newest first.
//
// Opening the panel is what marks it read, so the count is a number a
// person can clear by doing the thing the count is asking for.

import { For, Show, createSignal } from "solid-js";

import type { Notice } from "../core/belief";
import { useSay, useUi } from "../ui";

function Line(props: { readonly notice: Notice }) {
  const error = () => props.notice.error;
  return (
    <li class="border-b border-g2 px-base py-snug last:border-0">
      <div class="flex items-baseline gap-snug">
        <span class="font-label text-alert">{error().action}</span>
        <span class="font-mono text-note text-text-faint">{error().code}</span>
      </div>
      <div class="mt-tight break-all font-mono text-note text-text-quiet">{error().subject}</div>
      <Show when={error().recovery !== ""}>
        <div class="mt-tight text-note text-text-faint">{error().recovery}</div>
      </Show>
    </li>
  );
}

export function Notices() {
  const ui = useUi();
  const say = useSay();
  const [open, setOpen] = createSignal(false);
  const held = () => [...ui.conn.belief.notices].reverse();
  const unread = () => ui.conn.belief.notices.filter((notice) => !notice.seen).length;
  const show = () => {
    setOpen((was) => !was);
    ui.conn.markNoticesSeen();
  };

  return (
    <div class="relative">
      <button
        type="button"
        class="rounded-control px-snug py-tight text-label text-text-quiet hover:bg-g2"
        aria-expanded={open()}
        aria-label={
          unread() === 0 ? say("notices") : `${say("notices")} · ${say("notices_unread", { n: String(unread()) })}`
        }
        onClick={show}
      >
        {/* wording-ok: a bell, which every language draws the same way */}
        <span aria-hidden="true">🔔</span>
        <Show when={unread() > 0}>
          <span class="ml-tight rounded-pill bg-alert px-tight font-mono text-note text-g0">
            {String(unread())}
          </span>
        </Show>
      </button>
      <Show when={open()}>
        <div
          role="dialog"
          aria-label={say("notices")}
          class="absolute top-full right-0 z-20 mt-tight max-h-palette w-palette overflow-y-auto rounded-panel border border-g3 bg-g1 shadow-composer"
        >
          <Show
            when={held().length > 0}
            fallback={<p class="px-base py-snug text-note text-text-faint">{say("notices_none")}</p>}
          >
            <ul>
              <For each={held()}>{(notice) => <Line notice={notice} />}</For>
            </ul>
          </Show>
        </div>
      </Show>
    </div>
  );
}
