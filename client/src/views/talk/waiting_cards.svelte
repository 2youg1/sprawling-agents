<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts" module>
  // The cards themselves. Separate from the ask so the gallery can show
  // what a person waiting on a run sees without a run being blocked
  // (docs/frontend-method.md).
  //
  // An approval is one of the three things in this product that stop
  // and ask, so it is drawn in the one language they share (client-SPEC
  // 7C): the bar on the leading edge that `asks` declares, and a glyph
  // beside the words. The glyph is what survives a forced-colour mode;
  // the amber is only reinforcement.
  //
  // Identical questions are grouped by their cluster key so one answer
  // covers one question; a tainted item is never grouped.

  import { SvelteMap } from "svelte/reactivity";

  import type { ApprovalItem } from "../../wire";

  interface Group {
    readonly key: string;
    readonly first: ApprovalItem;
    readonly items: readonly ApprovalItem[];
  }

  function grouped(items: readonly ApprovalItem[]): Group[] {
    const groups = new SvelteMap<string, ApprovalItem[]>();
    for (const item of items) {
      const key = item.tainted
        ? `item:${String(item.id)}`
        : `${item.cluster_key.class}:${item.cluster_key.detail}`;
      const held = groups.get(key);
      if (held === undefined) {
        groups.set(key, [item]);
      } else {
        held.push(item);
      }
    }
    const drawn: Group[] = [];
    for (const [key, held] of groups) {
      const sorted = [...held].sort((a, b) => a.created - b.created);
      const first = sorted[0];
      // Every group holds the item that created it; an empty one is
      // drawn as nothing rather than as a card with no question.
      if (first === undefined) continue;
      drawn.push({ key, first, items: sorted });
    }
    return drawn.sort((a, b) => a.first.created - b.first.created);
  }
</script>

<script lang="ts">
  import { approve } from "../../core/commands";
  import { fill, say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { ago } from "../../core/time";
  import { ui } from "../../ui";
  import Button from "../parts/button.svelte";
  import Glyph from "../parts/glyph.svelte";
  import Asked from "./asked.svelte";

  interface Props {
    readonly items: readonly ApprovalItem[];
  }

  const { items }: Props = $props();

  const u = ui();
  const { lang } = u;

  const groups = $derived(grouped(items));

  function rule(group: Group, verdict: "allow" | "deny"): void {
    for (const each of group.items) {
      u.send(approve(each.id, verdict));
    }
  }
</script>

{#each groups as group (group.key)}
  <div
    class="asks my-base rounded-panel border border-alert/50 bg-raised py-base pr-pane"
    role="group"
    aria-label={say($lang, "wait_title")}
  >
    <div class="flex items-center justify-between text-note">
      <span class="flex min-w-0 items-center gap-tight text-alert">
        <Glyph name="hand" size="sm" />
        <span class="truncate">{fill(say($lang, "wait_from"), { actor: group.first.actor })}</span>
      </span>
      <span class="shrink-0 text-text-faint">{ago($lang, group.first.created, u.now())}</span>
    </div>
    <p class="my-snug text-body leading-relaxed">{group.first.action_desc}</p>
    <Asked locator={group.first.artifact} />
    <!-- "Deny" reads like "give the files back", and it is not: it
        stops the work and undoes nothing, so the card names the place
        that does bring a file back. -->
    <p class="mb-snug text-note text-text-faint">
      {say($lang, "wait_deny_keeps")}
      <a class="underline hover:text-text" href={toFragment({ kind: "record", lens: "bin" })}>
        {say($lang, "wait_bin")}
      </a>
    </p>
    <div class="flex flex-wrap items-center gap-snug text-note">
      {#if group.items.length > 1}
        <span class="text-text-faint">
          {fill(say($lang, "wait_same"), { n: String(group.items.length) })}
        </span>
      {/if}
      {#if group.first.tainted}
        <span class="rounded-pill bg-raised px-snug text-text-quiet">{say($lang, "wait_tainted")}</span>
      {/if}
      <span class="flex-1"></span>
      <Button
        label={say($lang, "wait_deny")}
        tone="quiet"
        onPress={() => {
          rule(group, "deny");
        }}
      />
      <Button
        label={say($lang, "wait_allow")}
        tone="primary"
        onPress={() => {
          rule(group, "allow");
        }}
      />
    </div>
  </div>
{/each}
