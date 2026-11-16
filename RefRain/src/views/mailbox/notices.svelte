<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  
  
  import { SvelteMap } from "svelte/reactivity";

  import type { Notice as NoticeRecord } from "../../core/belief";
  import { urgencyOf } from "../../core/deferral";
  import type { Lang } from "../../core/lang";
  import { say } from "../../core/lang";
  import { recoveryFor } from "../../core/recovering";
  import { ago, clock } from "../../core/time";
  import { ui } from "../../ui";
  import { recover, recoveryLabel, recoveryWhy } from "../notice_recovery";
  import Button from "../parts/button.svelte";
  import Notice from "../parts/notice.svelte";
  import Section from "./section.svelte";
  import { shown, sweep } from "./swept.svelte";

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;

  const visible = $derived(
    $belief.notices.filter((notice) => urgencyOf(notice.error) === "ordinary" && shown(notice)),
  );

  
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

  
  function dayOf(tongue: Lang, at: number, now: number): string {
    const day = new Date(at).toDateString();
    if (day === new Date(now).toDateString()) return say(tongue, "notices_today");
    if (day === new Date(now - 86_400_000).toDateString()) return say(tongue, "notices_yesterday");
    return clock(tongue, at);
  }
</script>

<Section title="notices" empty="notices_none" count={visible.length}>
  {#snippet tools()}
    {#if visible.length > 0}
      <Button
        tone="quiet"
        label={say($lang, "notices_clear")}
        onPress={() => {
          sweep(visible);
        }}
      />
    {/if}
  {/snippet}
  {#each days as day (day.id)}
    <h4 class="pt-snug text-note text-text-faint">{day.label}</h4>
    <ul>
      {#each day.items as notice (notice.key)}
        <li
          class="flex min-w-0 items-start gap-snug rounded-card focus-visible:wash"
          data-entry
          tabindex="-1"
        >
          <div class="min-w-0 flex-1">
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
                {#each recoveryFor(notice.error) as recovery (recoveryLabel(recovery, $lang))}
                  {@const why = recoveryWhy(u, recovery, notice)}
                  <Button
                    tone="quiet"
                    label={recoveryLabel(recovery, $lang)}
                    {...why === undefined ? {} : { why: say($lang, why) }}
                    onPress={() => {
                      recover(u, recovery, notice);
                    }}
                  />
                {/each}
              {/snippet}
            </Notice>
          </div>
          <kbd class="entry-n mt-snug" aria-hidden="true"></kbd>
        </li>
      {/each}
    </ul>
  {/each}
</Section>
