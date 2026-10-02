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
  // A design question is one of the things in this product that stop
  // and ask, so it is drawn on the one decide card they share
  // (`parts/decide.svelte`, client/Spec.lean §7C); this file gives the card its
  // body and its two answers, y to allow and n to deny.
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
  import Decide from "../parts/decide.svelte";
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
  <div class="my-base">
    <Decide
      kind="question"
      asker={fill(say($lang, "wait_from"), { actor: group.first.actor })}
      at={ago($lang, group.first.created, u.now())}
      choices={[
        {
          answer: "yes",
          label: say($lang, "wait_allow"),
          onPress: () => {
            rule(group, "allow");
          },
        },
        {
          answer: "no",
          label: say($lang, "wait_deny"),
          onPress: () => {
            rule(group, "deny");
          },
        },
      ]}
    >
      {#snippet body()}
        <p class="text-body leading-relaxed">{group.first.action_desc}</p>
        <Asked locator={group.first.artifact} />
        <!-- "Deny" reads like "give the files back", and it is not: it
            stops the work and undoes nothing, so the card names the place
            that does bring a file back. -->
        <p class="mt-snug text-note text-text-faint">
          {say($lang, "wait_deny_keeps")}
          <a class="underline hover:text-text" href={toFragment({ kind: "record", lens: "bin" })}>
            {say($lang, "wait_bin")}
          </a>
        </p>
        {#if group.items.length > 1 || group.first.tainted}
          <p class="mt-snug flex flex-wrap items-center gap-snug text-note">
            {#if group.items.length > 1}
              <span class="text-text-faint">
                {fill(say($lang, "wait_same"), { n: String(group.items.length) })}
              </span>
            {/if}
            {#if group.first.tainted}
              <span class="rounded-pill bg-raised px-snug text-text-quiet">{say($lang, "wait_tainted")}</span>
            {/if}
          </p>
        {/if}
      {/snippet}
    </Decide>
  </div>
{/each}
