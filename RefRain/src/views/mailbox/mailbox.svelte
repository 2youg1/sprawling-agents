<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
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
    readonly asked: number;

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

  let heard = 0;
  $effect(() => {
    const now = asked;
    if (now === heard) return;
    heard = now;
    feed("toggle");
  });

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

  $effect(() => {
    const row = wantedRow();
    if (row === null) return;
    takeRow();
    const next = stepLetter(
      { mail: { shown: open, focus: focusNow() }, opener: row, row: null },
      { kind: "close" },
    );
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

  const blocking = $derived(
    $belief.notices.filter((notice) => !notice.seen && urgencyOf(notice.error) === "needs_you")
      .length,
  );

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

{#if open}
  <aside
    bind:this={column}
    id="{uid}-column"
    aria-label={say($lang, "edge_mailbox")}
    class="push fixed inset-y-0 left-0 z-(--layer-mailbox) flex h-full w-[calc(440px+var(--spacing-margin)+var(--spacing-key)+var(--spacing-pane))] max-w-[100vw] flex-col overflow-hidden bg-raised pl-[calc(var(--spacing-margin)+var(--spacing-key)+var(--spacing-pane))] text-body text-text shadow-sheet narrow:w-full narrow:pl-0"
  >
    <Column
      onClose={() => {
        feed("follow");
      }}
    />
  </aside>
{/if}
