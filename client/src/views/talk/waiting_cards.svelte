<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts">
  // The cards themselves. Separate from the ask so the gallery can show
  // what a person waiting on a run sees without a run being blocked
  // (docs/frontend-method.md). A design question is drawn on the one
  // decide card the things that stop and ask share; this seat gives the
  // card its two answers, y to allow and n to deny, and one answer
  // rules every question of its group (`grouped`, `./waiting`).
  import { approve } from "../../core/commands";
  import { fill, say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { ago } from "../../core/time";
  import { ui } from "../../ui";
  import type { ApprovalItem, Locator } from "../../wire";
  import Asked from "./asked.svelte";
  import NotePlace from "./note_place.svelte";
  import Look from "./waiting_cards.look.svelte";
  import { grouped } from "./waiting";
  import type { Group, WaitingCardLook } from "./waiting";

  interface Props {
    readonly items: readonly ApprovalItem[];
  }

  const { items }: Props = $props();

  const u = ui();
  const { lang } = u;

  function rule(group: Group, verdict: "allow" | "deny"): void {
    for (const each of group.items) {
      u.send(approve(each.id, verdict));
    }
  }

  // "Deny" reads like "give the files back", and it is not: it stops
  // the work and undoes nothing, so the card names the place that does
  // bring a file back.
  const cards = $derived(
    grouped(items).map(
      (group): WaitingCardLook => ({
        key: group.key,
        asker: fill(say($lang, "wait_from"), { actor: group.first.actor }),
        at: ago($lang, group.first.created, u.now()),
        allow: { label: say($lang, "wait_allow"), onPress: () => { rule(group, "allow"); } },
        deny: { label: say($lang, "wait_deny"), onPress: () => { rule(group, "deny"); } },
        action: group.first.action_desc,
        locator: group.first.artifact,
        keeps: say($lang, "wait_deny_keeps"),
        bin: { text: say($lang, "wait_bin"), href: toFragment({ kind: "record", lens: "bin" }) },
        same: group.items.length > 1 ? fill(say($lang, "wait_same"), { n: String(group.items.length) }) : undefined,
        tainted: group.first.tainted ? say($lang, "wait_tainted") : undefined,
      }),
    ),
  );
</script>

{#snippet asked(locator: Locator)}
  <Asked {locator} />
{/snippet}

{#if cards.length > 0}
  <NotePlace rhythm="shape">
    <Look {cards} {asked} />
  </NotePlace>
{/if}
