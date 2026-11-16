

<script lang="ts">
  
  
  import { QUERIES } from "../../core/asking";
  import { ui } from "../../ui";
  import type { Address, RoundsAnswer, RunId } from "../../wire";
  import { barOf, contextOf, lockOfConfig, remindersOf } from "../talk/gauge";
  import Look from "./context_bar.look.svelte";

  interface Props {
    readonly room: Address;
    readonly run: RunId;
  }

  const { room, run }: Props = $props();

  const u = ui();
  const { lang } = u;
  const endpoints = u.conn.asking.ask(QUERIES.endpoints);
  const config = $derived(u.conn.asking.ask({ config: { addr: room } }));

  let rounds = $state<RoundsAnswer | undefined>(undefined);
  $effect(() => {
    const at = run;
    rounds = undefined;
    return u.conn.asking.ask({ rounds: { run: at } }).subscribe((held) => {
      rounds = held !== undefined && "rounds" in held ? held.rounds : undefined;
    });
  });

  const bar = $derived(
    barOf(
      $lang,
      contextOf(
        rounds?.turns ?? [],
        $endpoints !== undefined && "endpoints" in $endpoints ? $endpoints.endpoints.endpoints : [],
        remindersOf($config),
        lockOfConfig($config),
      ),
    ),
  );
</script>

{#if bar !== null}
  <span class="col-start-2 col-end-4 mt-snug"><Look {...bar} /></span>
{/if}
