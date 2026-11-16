<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  
  
  import { readAnswer } from "../../core/answered";
  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Address, ProposalCard } from "../../wire";
  import Button from "../parts/button.svelte";
  import Look from "./proposals.look.svelte";
  import Card from "./proposals_card.svelte";

  interface Props {
    readonly doc: Address;
    
    
    readonly unshowable: (card: ProposalCard) => string | undefined;
    
    readonly onShow: (card: ProposalCard) => void;
  }

  const { doc, unshowable, onShow }: Props = $props();

  const u = ui();
  const { lang } = u;

  const asked = $derived(u.conn.asking.ask({ proposals: doc }));
  const read = $derived(
    readAnswer($asked, (answer) => ("proposals" in answer ? answer.proposals : undefined)),
  );
  const open = $derived(read.kind === "held" ? read.value.open : []);
  const version = $derived(read.kind === "held" ? (read.value.version ?? null) : null);
  const id = $props.id();
</script>

{#if open.length > 0}
  <Look
    band={{ "aria-labelledby": `${id}-title` }}
    heading={{ id: `${id}-title` }}
    title={fill(say($lang, "proposals_title"), { n: String(open.length) })}
    {cards}
  />
{/if}

{#snippet cards()}
  {#each open as card (card.id)}
    <Card {doc} {card} {version}>
      {#snippet lead()}
        {@const why = unshowable(card)}
        <div class="flex justify-end">
          <Button
            tone="quiet"
            label={say($lang, "proposal_show")}
            {...why === undefined ? {} : { why }}
            onPress={() => {
              onShow(card);
            }}
          />
        </div>
      {/snippet}
    </Card>
  {/each}
{/snippet}
