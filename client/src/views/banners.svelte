<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The slot above every page, on the shell's grid: the page lost the
  // city, the city is halted, or both. Each says that nothing on this
  // page is moving and why, so they share one place a person learns to
  // look (`link_banner.svelte`).
  import { release } from "../core/commands";
  import { fill, say } from "../core/lang";
  import { cityIsShut, CITY } from "../core/scope";
  import { RELEASE_ALL } from "../core/slash";
  import { ui } from "../ui";
  import LinkBanner from "./link_banner.svelte";
  import Banner from "./parts/banner.svelte";
  import Button from "./parts/button.svelte";

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const linkState = u.conn.state;
  const unsent = u.conn.unsent;

  const halted = $derived(cityIsShut($belief.halted));
  const frozen = $derived($belief.cancelled);
  // The attempt the ladder is on since the link was lost, held through
  // each `opening` between two waits so the banner does not blink off
  // for every try; null while the page is live or has never been.
  let lostAttempt = $state<number | null>(null);
  $effect(() => {
    const now = $linkState;
    if (now.kind === "backoff") lostAttempt = now.attempt + 1;
    else if (now.kind === "live" || now.kind === "refused") lostAttempt = null;
  });
</script>

{#if lostAttempt !== null || halted}
  <div class="col-[2/12] row-start-1 flex flex-col gap-snug pb-base narrow:col-span-full">
    {#if lostAttempt !== null}
      <LinkBanner attempt={lostAttempt} unsent={$unsent} onRetry={u.conn.retry} />
    {/if}
    {#if halted}
      <Banner
        text={say($lang, "halt_title")}
        {...frozen > 0 ? { detail: fill(say($lang, "halt_frozen"), { n: String(frozen) }) } : {}}
        weight="alert"
      >
        {#snippet action()}
          <Button label={RELEASE_ALL} tone="secondary" onPress={() => u.send(release(CITY))} />
        {/snippet}
      </Banner>
    {/if}
  </div>
{/if}
