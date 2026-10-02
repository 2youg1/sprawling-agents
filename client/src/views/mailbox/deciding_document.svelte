<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The proposal cards open on one document, in the mailbox's deciding
  // section (client/Spec.lean §4-55): each card leads with the document's path
  // and the way to it, which opens the document on the right side with
  // the same cards above its text.
  import { readAnswer } from "../../core/answered";
  import { say } from "../../core/lang";
  import { buildingOf } from "../../core/route";
  import type { Address } from "../../wire";
  import { ui } from "../../ui";
  import { openDocument } from "../inspect/open.svelte";
  import Button from "../parts/button.svelte";
  import Card from "../refrain/proposals_card.svelte";

  interface Props {
    readonly doc: Address;
    // Puts the mailbox away once the person follows a card to its
    // document.
    readonly onLeave: () => void;
  }

  const { doc, onLeave }: Props = $props();

  const u = ui();
  const { lang } = u;

  const asked = $derived(u.conn.asking.ask({ proposals: doc }));
  const read = $derived(readAnswer($asked, (answer) => ("proposals" in answer ? answer.proposals : undefined)));
  const building = $derived(buildingOf(doc));
  const path = $derived(doc.slice(building.length + 1));
</script>

{#if read.kind === "held"}
  {#each read.value.open as card (card.id)}
    <div class="my-base">
      <Card {doc} {card} version={read.value.version ?? null}>
        {#snippet lead()}
          <div class="flex min-w-0 items-center gap-snug">
            <span class="min-w-0 flex-1 truncate font-mono text-note text-text-quiet" title={doc}>{doc}</span>
            <Button
              tone="quiet"
              label={say($lang, "proposal_open")}
              onPress={() => {
                openDocument({ building, path, version: null });
                onLeave();
              }}
            />
          </div>
        {/snippet}
      </Card>
    </div>
  {/each}
{/if}
