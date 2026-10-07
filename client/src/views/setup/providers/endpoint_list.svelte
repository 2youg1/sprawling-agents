<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The seat of the list of what is attached: it asks the city for the
  // retry count an endpoint calls with when none is chosen, lends each
  // row its account editor (`./accounts.svelte`), and draws whatever
  // `./endpoint_list.look.svelte` is. What a row says is
  // `./endpoint_list`; `../providers.ts` is the one specifier a caller
  // reaches this list and the attach form through.

  import type { EndpointsAnswer } from "../../../wire";

  export interface EndpointListProps {
    readonly answer: EndpointsAnswer;
  }
</script>

<script lang="ts">
  import { ui } from "../../../ui";
  import { HALL } from "../../shared/buildings";
  import Accounts from "./accounts.svelte";
  import { lookOf } from "./endpoint_list";
  import Look from "./endpoint_list.look.svelte";
  import { endpointRoster } from "./rosters";

  const { answer }: EndpointListProps = $props();
  const u = ui();
  const { lang } = u;
  // The retry count an endpoint calls with when none is chosen is the
  // same at every address, and the hall is the one every city has.
  const config = u.conn.asking.ask({ config: { addr: HALL } });
  const fallback = $derived.by(() => {
    const held = $config;
    return held !== undefined && "config" in held ? held.config.tuning.account_retries : undefined;
  });

  const look = $derived(lookOf(answer, $lang));
</script>

{#snippet accounts(name: string)}
  {@const endpoint = answer.endpoints.find((each) => each.name === name)}
  {#if endpoint !== undefined}
    <Accounts roster={endpointRoster(endpoint, fallback)} />
  {/if}
{/snippet}

<Look {...look} {accounts} />
