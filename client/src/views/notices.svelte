<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The notice drawer: one dot at the top of the rail opens a
  // full-height drawer that keeps everything the city has to tell this
  // person - the link, the questions waiting on them, and every refusal
  // of the session, newest first and grouped by day (client-SPEC
  // 4-35). Opening it is what marks the refusals read, so the count on
  // the dot is a number a person clears by doing the thing the count
  // asks for.
  //
  // **The drawer is a `popover`, which is why it is the one seat that
  // may anchor to the viewport.** `popover="auto"` gives the top layer,
  // light dismiss and Escape as platform behaviour, so this file holds
  // no scrim, no key handling and no stacking number (design 4-21);
  // design 4-23's objection to the Popover API is about anchoring a
  // panel to a control in the flow, which a viewport-anchored drawer
  // never needs.
  //
  // **What a recovery's control says, when it may be pressed, and what
  // pressing it does is `notice_recovery.ts`'s** - one answer, read by
  // this seat and by the toast alike (client-SPEC 4-35).

  import { SvelteMap } from "svelte/reactivity";

  import type { Notice as NoticeRecord } from "../core/belief";
  import type { Lang } from "../core/lang";
  import { fill, say } from "../core/lang";
  import type { Rail } from "../core/prefs";
  import { linkRecovery, recoveryFor } from "../core/recovering";
  import { ago, clock } from "../core/time";
  import { ui } from "../ui";
  import { recover, recoveryLabel, recoveryWhy } from "./notice_recovery";
  import Button from "./parts/button.svelte";
  import Empty from "./parts/empty.svelte";
  import Notice from "./parts/notice.svelte";
  import { WaitingCards } from "./talk/waiting.svelte";

  interface Props {
    // Which edge the drawer hangs off: the rail's right edge moves with
    // the rail's posture, and this is the drawer's whole anchor.
    readonly posture: Rail;
  }

  const { posture }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const link = u.conn.state;
  const approvals = u.approvals;

  let open = $state(false);
  let drawer = $state<HTMLElement | undefined>(undefined);
  // What `clear` swept, by notice key and the count it swept. A refusal
  // that arrives again is news again, so a repeat past the swept count
  // shows; nothing here deletes the city's answer, which stays in the
  // belief for a person who scrolls back.
  let swept = $state.raw<Readonly<Record<string, number>>>({});

  const EDGE: Readonly<Record<Rail, string>> = {
    named: "left-[var(--spacing-rail-open)]",
    glyphs: "left-[var(--spacing-rail)]",
    away: "left-0",
  };

  // The dot, in the same square a glyph is drawn in, so the first mark
  // of every rail row starts at the same x.
  const DOT = "flex size-glyph shrink-0 items-center justify-center";

  function toggle(): void {
    const node = drawer;
    if (node === undefined) return;
    if (node.matches(":popover-open")) {
      node.hidePopover();
      return;
    }
    node.showPopover();
    open = true;
    // Reading the bell is what marks it read: an unread count that
    // survived the drawer being open would be a number nobody clears.
    u.conn.markNoticesSeen();
  }

  function sweep(): void {
    const next: Record<string, number> = { ...swept };
    for (const notice of $belief.notices) {
      next[notice.key] = notice.count;
    }
    swept = next;
  }

  // What the dot says when it is all the rail has room for: what it is,
  // then each reason it is not grey. `aria-label` is the collapsed
  // control's whole name, so it carries the state as well.
  const name = $derived.by(() => {
    const parts = [say($lang, "presence_title")];
    const waiting = $approvals.length;
    const unread = $belief.notices.filter((notice) => !notice.seen).length;
    if (waiting > 0) parts.push(fill(say($lang, "nav_waiting"), { n: String(waiting) }));
    if (unread > 0) parts.push(fill(say($lang, "notices_unread"), { n: String(unread) }));
    return parts.join(" · ");
  });

  const busy = $derived(
    $approvals.length > 0 || $belief.notices.some((notice) => !notice.seen),
  );

  // A link still on its way up is the one mark that resolves by itself,
  // so it moves rather than asking for anything.
  const connecting = $derived($link.kind !== "live" && $link.kind !== "refused");

  // Four of the six link states are one word to a person: the socket is
  // on its way up.
  const linkWord = $derived.by((): string => {
    switch ($link.kind) {
      case "live":
        return say($lang, "link_live");
      case "refused":
        return say($lang, "link_refused");
      case "idle":
      case "opening":
      case "handshaking":
      case "backoff":
        return say($lang, "link_connecting");
    }
  });

  const visible = $derived(
    $belief.notices.filter((notice) => notice.count > (swept[notice.key] ?? 0)),
  );

  // The notices as they are read: newest first, grouped by the day the
  // refusal first arrived. A repeat keeps its first-seen stamp, so the
  // day a refusal belongs to never moves.
  const days = $derived.by(() => {
    const now = u.now();
    const groups = new SvelteMap<string, NoticeRecord[]>();
    for (const notice of [...visible].reverse()) {
      const id = new Date(notice.at).toDateString();
      const held = groups.get(id) ?? [];
      held.push(notice);
      groups.set(id, held);
    }
    return [...groups.entries()].map(([id, items]) => ({
      id,
      label: dayOf($lang, items[0]?.at ?? 0, now),
      items,
    }));
  });

  // A day is named by the two days a person has words for and dated
  // beyond them. The date is data, formatted where every other time on
  // this client is formatted (`core/time`).
  function dayOf(tongue: Lang, at: number, now: number): string {
    const day = new Date(at).toDateString();
    if (day === new Date(now).toDateString()) return say(tongue, "notices_today");
    if (day === new Date(now - 86_400_000).toDateString()) return say(tongue, "notices_yesterday");
    return clock(tongue, at);
  }
</script>

<!-- One dot and nothing beside it: the rail is 44 px wide when it is
collapsed, and what the dot means is in the drawer rather than in a
second mark next to it. -->
{#snippet dot()}
  <span class={DOT}>
    <span
      class={[
        "inline-block size-dot rounded-pill",
        busy ? "bg-alert" : "bg-mark",
        connecting ? "animate-pulse" : "",
      ]}
    ></span>
  </span>
{/snippet}

<button
  type="button"
  class="flex h-rail w-full items-center px-base text-label text-text-quiet hover:text-text"
  aria-expanded={open}
  aria-label={name}
  onclick={toggle}
>
  <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
  {@render dot()}
</button>

<aside
  bind:this={drawer}
  popover="auto"
  aria-label={say($lang, "presence_title")}
  class="slide fixed inset-y-0 right-auto m-0 h-full w-[440px] max-w-[100vw] overflow-y-auto bg-raised shadow-sheet {EDGE[posture]}"
  ontoggle={(event) => {
    open = event.currentTarget.matches(":popover-open");
  }}
>
  <div class="flex items-center gap-snug border-b border-edge px-base py-snug text-note">
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
    {@render dot()}
    <span class="text-text">{linkWord}</span>
    {#if $link.kind === "refused"}
      {@const lever = linkRecovery($link.error.code)}
      <Button
        tone="quiet"
        label={recoveryLabel(lever, $lang)}
        onPress={() => {
          recover(u, lever, null);
        }}
      />
    {/if}
    <span class="flex-1"></span>
    <Button
      tone="quiet"
      label={say($lang, "notices_mark_all")}
      onPress={() => {
        u.conn.markNoticesSeen();
      }}
    />
    <Button tone="quiet" label={say($lang, "notices_clear")} onPress={sweep} />
  </div>

  {#if $approvals.length > 0}
    <section class="border-b border-edge px-base py-snug" aria-label={say($lang, "talk_waiting_you")}>
      <h2 class="text-label font-label text-alert">{say($lang, "talk_waiting_you")}</h2>
      <WaitingCards items={$approvals} />
    </section>
  {/if}

  <section aria-label={say($lang, "notices")}>
    <h2 class="px-base pt-snug text-label font-label text-text-quiet">{say($lang, "notices")}</h2>
    {#if visible.length === 0}
      <Empty missing="notices_none" />
    {:else}
      {#each days as day (day.id)}
        <h3
          class="sticky top-0 bg-raised px-base py-tight text-note text-text-faint"
        >
          {day.label}
        </h3>
        <ul>
          {#each day.items as notice (notice.key)}
            <li>
              <Notice
                seat="drawer"
                weight="alert"
                action={notice.error.action}
                code={notice.error.code}
                subject={notice.error.subject}
                recovery={notice.error.recovery}
                at={ago($lang, notice.at, u.now())}
                count={notice.count}
              >
                {#snippet actions()}
                  {#each recoveryFor(notice.error.code) as recovery (recoveryLabel(recovery, $lang))}
                    {@const why = recoveryWhy(u, recovery, notice.about)}
                    <Button
                      tone="quiet"
                      label={recoveryLabel(recovery, $lang)}
                      {...why === undefined ? {} : { why: say($lang, why) }}
                      onPress={() => {
                        recover(u, recovery, notice.about);
                      }}
                    />
                  {/each}
                {/snippet}
              </Notice>
            </li>
          {/each}
        </ul>
      {/each}
    {/if}
  </section>
</aside>
