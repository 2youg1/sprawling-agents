<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The proposal cards open on one document, in the mailbox's deciding
  // section (client/Spec.lean §4-55): each card leads with the document's
  // whole path, when its newest card was offered, and the way to it, which opens the
  // document on the right side with the same cards above its text, and
  // the way to the letter, which opens this one card on the right side
  // with the text before and after it (client D73).
  import { readAnswer } from "../../core/answered";
  import { say } from "../../core/lang";
  import { buildingOf } from "../../core/route";
  import { ago } from "../../core/time";
  import type { Address, TimeMs } from "../../wire";
  import { ui } from "../../ui";
  import { openDocument, openLetter } from "../inspect/open.svelte";
  import Button from "../parts/button.svelte";
  import Card from "../refrain/proposals_card.svelte";
  import { WHY } from "./pending";
  import { returnTo } from "./returning.svelte";

  interface Props {
    readonly doc: Address;
    // When the newest card on it was offered, `null` when the city could
    // not say.
    readonly at: TimeMs | null;
    // Puts the mailbox away once the person follows a card to its
    // document.
    readonly onLeave: () => void;
  }

  const { doc, at, onLeave }: Props = $props();

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
          <p class="text-note text-text-quiet">{say($lang, WHY.proposal)}</p>
          <!-- The whole path, wrapped where it must: a path cut short
          keeps the part that tells two documents apart out of sight, and
          a hint on words nobody can focus reaches only a pointer. -->
          <p class="font-mono text-note wrap-anywhere text-text-quiet">{doc}</p>
          <div class="flex min-w-0 items-center gap-snug">
            <span class="min-w-0 flex-1 text-note text-text-faint">{at === null ? "" : ago($lang, at, u.now())}</span>
            <span data-letter={card.id} class="contents">
              <Button
                tone="quiet"
                label={say($lang, "letter_open")}
                onPress={() => {
                  openLetter({ doc, card: card.id }, () => {
                    returnTo(card.id);
                  });
                  onLeave();
                }}
              />
            </span>
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
