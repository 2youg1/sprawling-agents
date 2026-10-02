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
  // **The column is a `popover`**, which is why it may anchor to the
  // viewport: `popover="auto"` gives the top layer, light dismiss,
  // Escape, and the focus back on this key, all as platform behaviour,
  // so this file holds no scrim and no stacking number (design 4-21).
  // What it holds is `column.svelte`, mounted only while it is open, so
  // a closed mailbox asks the city nothing.
  import { urgencyOf } from "../../core/deferral";
  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import { EDGE_KEY } from "../edge.svelte";
  import Glyph from "../parts/glyph.svelte";
  import Tip from "../parts/tip.svelte";
  import Column from "./column.svelte";
  import { openCards, proposedDocs } from "./deciding_proposals";
  import { linkWord } from "./link_word";

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

  // The chord answers here, where the column is: each new ask toggles it.
  let heard = 0;
  $effect(() => {
    const now = asked;
    if (now === heard) return;
    heard = now;
    toggle();
  });

  function toggle(): void {
    const node = column;
    if (node === undefined) return;
    if (node.matches(":popover-open")) {
      node.hidePopover();
      return;
    }
    node.showPopover();
  }

  function close(): void {
    column?.hidePopover();
  }

  // What needs the person, and what is merely new.
  const blocking = $derived(
    $belief.notices.filter((notice) => !notice.seen && urgencyOf(notice.error) === "needs_you").length,
  );
  // The cards open on the documents this page saw proposals for: the
  // only questions a closed mailbox asks (4-49, 4-55).
  const offered = $derived($belief.proposed);
  const proposed = $derived(openCards(u.conn.asking, proposedDocs(offered)));
  const needs = $derived(($approvals?.length ?? 0) + blocking + $proposed);
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
      aria-label={name}
      aria-describedby={id}
      popovertarget="{uid}-column"
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
            connecting ? "animate-pulse bg-text-quiet" : "bg-alert",
          ]}
          aria-hidden="true"
        ></span>
      {/if}
    </button>
  {/snippet}
</Tip>

<!-- The key is the column's invoker (`popovertarget`), so a press on it
while the column is open closes it rather than being read as a click
outside that closes it and then a press that opens it again. -->
<aside
  bind:this={column}
  id="{uid}-column"
  popover="auto"
  aria-label={say($lang, "edge_mailbox")}
  class="slide fixed inset-y-0 right-auto left-[calc(var(--spacing-margin)+var(--spacing-key)+var(--spacing-pane))] m-0 open:flex h-full w-[440px] max-w-[100vw] flex-col overflow-hidden bg-raised text-body text-text shadow-sheet narrow:left-0 narrow:w-full"
  ontoggle={(event) => {
    open = event.currentTarget.matches(":popover-open");
    // Opening the mailbox is reading it: an unread mark that outlived
    // the column being open would be a mark nobody clears.
    if (open) u.conn.markNoticesSeen();
  }}
>
  {#if open}
    <Column onClose={close} />
  {/if}
</aside>
