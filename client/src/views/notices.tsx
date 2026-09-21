// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One dot, at the top of the rail, for everything the city has to tell
// this person right now: the link, the questions waiting on them, and
// what was refused.
//
// It used to be two marks in two corners. A blue dot on the rail
// reported that the socket was live, which is the state a person is in
// for the whole day and therefore the one state worth no colour at
// all; a bell floated over the top right corner and counted refusals.
// Neither could say what the other knew, so a city that was waiting on
// an answer looked exactly like a city that was idle.
//
// **The dot has two states, and the second one is a question.** Grey
// is nothing to do. Yellow means at least one of three things is true
// - somebody is waiting on an answer, a refusal has not been read, or
// the link is not live - and the panel under the dot says which. A
// link that is still connecting pulses, because that one resolves by
// itself and the others do not.
//
// **A refusal a person waved away is still the answer to what they
// asked.** Dismissing one used to destroy the only copy. The corner
// still carries the newest one, because that is the one they are
// waiting for; everything that reached the corner is here afterwards,
// newest first. Opening the panel is what marks it read, so the count
// is a number a person can clear by doing the thing the count asks
// for.

import { For, Show, createSignal } from "solid-js";

import type { Notice } from "../core/belief";
import { useApprovals, useSay, useUi } from "../ui";
import { WaitingCards } from "./talk/waiting";

// A dot drawn in the same box a glyph is drawn in, so every row on the
// rail starts its first mark at the same x. It used to be centred in
// its own smaller box, five pixels left of every icon under it.
export function Dot(props: { readonly tone: string }) {
  return (
    <span class="flex size-glyph shrink-0 items-center justify-center">
      <span class={`inline-block size-dot rounded-pill ${props.tone}`} />
    </span>
  );
}

function Line(props: { readonly notice: Notice }) {
  const error = () => props.notice.error;
  return (
    <li class="border-b border-edge px-base py-snug last:border-0">
      <div class="flex items-baseline gap-snug">
        <span class="font-label text-alert">{error().action}</span>
        <span class="font-mono text-note text-text-faint">{error().code}</span>
      </div>
      <div class="mt-tight wrap-anywhere font-mono text-note text-text-quiet">{error().subject}</div>
      <Show when={error().recovery !== ""}>
        <div class="mt-tight text-note text-text-faint">{error().recovery}</div>
      </Show>
    </li>
  );
}

export function Presence() {
  const ui = useUi();
  const say = useSay();
  const [open, setOpen] = createSignal(false);
  const approvals = useApprovals();
  const held = () => [...ui.conn.belief.notices].reverse();
  const unread = () => ui.conn.belief.notices.filter((notice) => !notice.seen).length;
  const link = () => ui.conn.state().kind;

  // The word for where the link stands. Four of the six states are one
  // word to a person: the socket is on its way up.
  const linkWord = () => {
    switch (link()) {
      case "live":
        return say("link_live");
      case "refused":
        return say("link_refused");
      case "idle":
      case "opening":
      case "handshaking":
      case "backoff":
        return say("link_connecting");
    }
  };
  const busy = () => approvals().length > 0 || unread() > 0;
  const tone = () => (busy() ? "bg-alert" : "bg-mark");
  // Collapsed, the dot is the only thing on this control, so its name
  // has to carry the state as well: what it is, then each of the two
  // reasons it is yellow.
  const name = () => {
    const parts = [say("presence_title")];
    if (approvals().length > 0) parts.push(say("nav_waiting", { n: String(approvals().length) }));
    if (unread() > 0) parts.push(say("notices_unread", { n: String(unread()) }));
    return parts.join(" · ");
  };
  const show = () => {
    setOpen((was) => !was);
    ui.conn.markNoticesSeen();
  };

  return (
    <div
      class="relative shrink-0"
      onKeyDown={(event) => {
        if (event.key === "Escape" && open()) {
          event.stopPropagation();
          setOpen(false);
        }
      }}
    >
      {/* One dot and nothing beside it: the rail's own width is 44 px
          when it is collapsed, and what the dot means is in the panel
          rather than in a second mark next to it. */}
      <button
        type="button"
        // `w-full`, because that is what the eight rows under it are.
        // This one said `w-rail` and came out a pixel wide of every
        // one of them; taking the width off entirely made it shrink to
        // its dot and come out two pixels short, which was worse. The
        // authority is the rows: a column of entries that fill their
        // holder has one member that states a width of its own, and
        // the member is the one that is wrong. `xtask render --survey`
        // put it as seven of eight agreeing, which is how a typing
        // mistake is told from a column that never had a consensus.
        class="flex h-rail w-full items-center px-base text-label text-text-quiet hover:text-text"
        aria-expanded={open()}
        aria-label={name()}
        onClick={show}
      >
        <Dot tone={tone()} />
      </button>
      <Show when={open()}>
        <div
          role="dialog"
          aria-label={say("presence_title")}
          class="absolute top-0 left-full z-20 ml-tight max-h-palette w-palette overflow-y-auto rounded-panel border border-edge-panel bg-raised shadow-composer"
        >
          <div class="flex items-center gap-snug border-b border-edge px-base py-snug text-note">
            <Dot tone={tone()} />
            <span class="text-text">{linkWord()}</span>
            <Show when={link() === "refused"}>
              <button
                type="button"
                class="ml-auto rounded-control px-base py-tight font-mono text-note text-accent hover:bg-raised"
                onClick={() => {
                  ui.conn.retry();
                }}
              >
                {say("link_retry")}
              </button>
            </Show>
          </div>
          <Show when={approvals().length > 0}>
            <section class="border-b border-edge px-base py-snug" aria-label={say("wait_title")}>
              <h2 class="text-label font-label text-alert">{say("wait_title")}</h2>
              <WaitingCards items={approvals()} />
            </section>
          </Show>
          <section aria-label={say("notices")}>
            <h2 class="px-base pt-snug text-label font-label text-text-quiet">{say("notices")}</h2>
            <Show
              when={held().length > 0}
              fallback={<p class="px-base py-snug text-note text-text-faint">{say("notices_none")}</p>}
            >
              <ul>
                <For each={held()}>{(notice) => <Line notice={notice} />}</For>
              </ul>
            </Show>
          </section>
        </div>
      </Show>
    </div>
  );
}
