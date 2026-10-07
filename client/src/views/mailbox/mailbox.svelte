<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The mailbox: the edge key at the foot of the first column and the
  // transient column it opens from the left (client/Spec.lean §4-49, docs/frontend-method.md §7E). One
  // column, ordered by what needs the person: deciding, working, recent,
  // and under them the notices the drawer used to hold (4-35).
  //
  // **The key says three things and keeps them apart** (refrain 3-6):
  // a number is what needs the person - questions, a door waiting on
  // their hand, a failure that stops the work - and nothing else; an
  // ordinary refusal nobody has read is a plain mark with no number,
  // which is all `core/deferral.ts` lets it do until a moment comes; and
  // the link, when it is not live, is a bar of its own whose name the
  // key's name says, never a share of the count.
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

  import { urgencyOf } from "../../core/deferral";
  import { fill, say } from "../../core/lang";
  import { markEndAtFrame, markStart } from "../../core/timing";
  import { ui } from "../../ui";
  import { EDGE_KEY } from "../edge.svelte";
  import Glyph from "../parts/glyph.svelte";
  import Tip from "../parts/tip.svelte";
  import Column from "./column.svelte";
  import { openCards } from "./deciding_proposals";
  import { stepLetter, stepMail } from "./layer";
  import type { MailFocus, MailInput } from "./layer";
  import { linkWord } from "./link_word";
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
  let key = $state<HTMLButtonElement | undefined>(undefined);

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

  const connecting = $derived($link.kind !== "live" && $link.kind !== "refused");

  // What the key says, which is its whole name: what it is, then each
  // reason it carries a mark.
  const name = $derived.by(() => {
    const parts = [say($lang, "edge_mailbox")];
    if (needs > 0) parts.push(fill(say($lang, "nav_waiting"), { n: String(needs) }));
    else if (fresh) parts.push(say($lang, "mailbox_new"));
    if ($link.kind !== "live") parts.push(linkWord($lang, $link));
    return parts.join(" · ");
  });
</script>

<Tip text={hint(name)} side="right" exposable>
  {#snippet children(id)}
    <button
      type="button"
      class={EDGE_KEY}
      aria-expanded={open}
      aria-controls="{uid}-column"
      aria-label={name}
      aria-describedby={id}
      bind:this={key}
      onclick={() => {
        feed("toggle");
      }}
    >
      <Glyph name="inbox" size="key" />
      {#if needs > 0}
        <span
          class="absolute -top-tight -right-[6px] min-w-[18px] rounded-pill bg-alert px-[5px] text-center text-tally leading-[18px] font-label text-on-accent"
          aria-hidden="true"
        >
          {needs}
        </span>
      {:else if fresh}
        <span class="absolute top-[6px] right-[6px] size-[6px] rounded-pill bg-text-quiet" aria-hidden="true"></span>
      {/if}
      {#if $link.kind !== "live"}
        <!-- The link's own mark: the bar `core/mark.ts` draws on the tab
        for a page that is not being told, here at the key's foot. -->
        <span
          class={[
            "absolute bottom-[6px] left-1/2 h-[2px] w-[10px] -translate-x-1/2 rounded-pill",
            connecting ? "pulse bg-text-quiet" : "bg-alert",
          ]}
          aria-hidden="true"
        ></span>
      {/if}
    </button>
  {/snippet}
</Tip>

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
