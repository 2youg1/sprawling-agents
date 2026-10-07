<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The mailbox: the edge key at the foot of the first column and the
  // transient column it opens from the left (client/Spec.lean §4-49,
  // docs/frontend-method.md §7E). One column, ordered by what needs the
  // person: deciding, working, recent, and under them the notices the
  // drawer used to hold (4-35). What the key says and which marks it
  // carries is `./key.ts`'s; it is drawn by the edge key look.
  //
  // **The column is a layer under the keys, not a popover** (client
  // D60): it pushes in from the left edge and the three edge keys stay
  // above it, which a top-layer popover would forbid. So what the
  // platform used to decide - Escape, a press outside, the focus back on
  // this key - is decided by `layer.ts`, the step the Lean model proves,
  // and this file only feeds it what happened and where the focus is.
  // What it holds is `column.svelte`, mounted only while it is open, so
  // a closed mailbox asks the city nothing.
  import { tick } from "svelte";
  import { createAttachmentKey } from "svelte/attachments";

  import { urgencyOf } from "../../core/deferral";
  import { say } from "../../core/lang";
  import { markEndAtFrame, markStart } from "../../core/timing";
  import { ui } from "../../ui";
  import Look from "../edge_key.look.svelte";
  import Column from "./column.svelte";
  import { openCards } from "./deciding_proposals";
  import { mailboxKey } from "./key";
  import { stepLetter, stepMail } from "./layer";
  import type { MailFocus, MailInput } from "./layer";
  import { takeRow, wantedRow } from "./returning.svelte";

  interface Props {
    // How many times the mailbox chord asked for the column: a count
    // rather than a flag, so a second press while it is open is heard
    // and closes it.
    readonly asked: number;
    // The key's hint, given its name: the edge writes the chord beside
    // it, from the one key table.
    readonly hint: (words: string) => string;
  }

  const { asked, hint }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const link = u.conn.state;
  const approvals = u.approvals;

  const uid = $props.id();
  let open = $state(false);
  let column = $state<HTMLElement | undefined>(undefined);
  let key: HTMLElement | undefined;
  const HOLD = createAttachmentKey();
  const hold = (node: HTMLElement): (() => void) => {
    key = node;
    return () => {
      if (key === node) key = undefined;
    };
  };

  // The chord answers here, where the column is: each new ask toggles it.
  let heard = 0;
  $effect(() => {
    const now = asked;
    if (now === heard) return;
    heard = now;
    feed("toggle");
  });

  // Where the focus is now, read from the page rather than remembered,
  // so a Tab out of the column is seen without a listener of its own.
  function focusNow(): MailFocus {
    const at = document.activeElement;
    if (at !== null && column?.contains(at) === true) return "inside";
    return at === key ? "key" : "elsewhere";
  }

  function feed(input: MailInput): void {
    const focus = focusNow();
    const next = stepMail({ shown: open, focus }, input);
    const opening = next.shown && !open;
    if (opening) {
      markStart("mailbox_open");
      u.conn.markNoticesSeen();
    }
    open = next.shown;
    if (opening) markEndAtFrame("mailbox_open");
    if (next.focus === "key" && focus !== "key") key?.focus();
  }

  // A press anywhere but the column and its own key puts it away; the
  // key's press is its click, which toggles.
  // A letter closed on the right side: the column comes back with the
  // focus on the row that opened it (client D73).
  $effect(() => {
    const row = wantedRow();
    if (row === null) return;
    takeRow();
    const next = stepLetter({ mail: { shown: open, focus: focusNow() }, opener: row, row: null }, { kind: "close" });
    open = next.mail.shown;
    const target = next.row;
    if (target === null) return;
    void tick().then(() => {
      column?.querySelector<HTMLElement>(`[data-letter="${CSS.escape(target)}"] button`)?.focus();
    });
  });

  function pressed(event: PointerEvent): void {
    const target = event.target;
    if (!(target instanceof Node)) return;
    if (column?.contains(target) === true || key?.contains(target) === true) return;
    feed("outside");
  }

  // Escape is the column's while it is open, unless something inside it
  // that has its own Escape - a menu, a popover - is open and takes it.
  function escaped(event: KeyboardEvent): void {
    if (event.key !== "Escape" || event.isComposing || event.defaultPrevented) return;
    if (column !== undefined && column.querySelector(":popover-open") !== null) return;
    event.preventDefault();
    feed("escape");
  }

  $effect(() => {
    if (!open) return;
    document.addEventListener("pointerdown", pressed, true);
    window.addEventListener("keydown", escaped);
    return () => {
      document.removeEventListener("pointerdown", pressed, true);
      window.removeEventListener("keydown", escaped);
    };
  });

  // What needs the person, and what is merely new.
  const blocking = $derived(
    $belief.notices.filter((notice) => !notice.seen && urgencyOf(notice.error) === "needs_you").length,
  );
  // The cards open on the documents this page saw proposals for: the
  // only questions a closed mailbox asks (4-49, 4-55).
  const proposed = openCards(u.conn.asking);
  const needs = $derived(($approvals?.length ?? 0) + blocking + $proposed.length);
  const fresh = $derived(
    $belief.notices.some((notice) => !notice.seen && urgencyOf(notice.error) === "ordinary"),
  );

  const look = $derived(
    mailboxKey({ needs, fresh, link: $link }, $lang, hint, {
      "aria-expanded": open,
      "aria-controls": `${uid}-column`,
      onclick: () => {
        feed("toggle");
      },
      [HOLD]: hold,
    }),
  );
</script>

<Look {...look} />

<!-- The column stands from the left edge of the frame, under the keys:
it is a fixed child of the edge keys' nav, whose stacking it shares, and
`-z-10` sets it under the keys inside that nav. Its padding keeps every
row clear of the key column, so the keys never cover what it says. -->
{#if open}
  <aside
    bind:this={column}
    id="{uid}-column"
    aria-label={say($lang, "edge_mailbox")}
    class="push fixed inset-y-0 left-0 -z-10 flex h-full w-[calc(440px+var(--spacing-margin)+var(--spacing-key)+var(--spacing-pane))] max-w-[100vw] flex-col overflow-hidden bg-raised pl-[calc(var(--spacing-margin)+var(--spacing-key)+var(--spacing-pane))] text-body text-text shadow-sheet narrow:w-full narrow:pl-0"
  >
    <Column
      onClose={() => {
        feed("follow");
      }}
    />
  </aside>
{/if}
