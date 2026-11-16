<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
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
  const closing = u.conn.closing;
  const closed = $derived($linkState.kind === "closed");
  const closingWords = $derived.by(() => {
    if ($closing !== "drain") return say($lang, "close_interrupting");
    const going = $belief.live.length;
    return going === 0
      ? say($lang, "close_closing")
      : fill(say($lang, "close_draining"), { n: String(going) });
  });

  const halted = $derived(cityIsShut($belief.halted));
  const frozen = $derived($belief.cancelled);
  let lostAttempt = $state<number | null>(null);
  $effect(() => {
    const now = $linkState;
    if (now.kind === "backoff") lostAttempt = now.attempt + 1;
    else if (now.kind === "live" || now.kind === "refused" || now.kind === "closed")
      lostAttempt = null;
  });
</script>

{#if lostAttempt !== null || halted || closed || $closing !== null}
  <div class="col-[2/12] row-start-1 flex flex-col gap-snug pb-base narrow:col-span-full">
    {#if closed}
      <Banner text={say($lang, "close_done")} weight="alert" />
    {:else if $closing !== null}
      <Banner text={closingWords} weight="notice" />
    {/if}
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
