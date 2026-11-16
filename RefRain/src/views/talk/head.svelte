<script lang="ts">
  import { fill, say } from "../../core/lang";
  import { hhmmss, isoInstant } from "../../core/time";
  import { ui } from "../../ui";
  import Look from "./head.look.svelte";
  import Sparkline from "./sparkline.svelte";
  import { tookWords } from "./timing";
  import type { Took } from "./timing";

  interface Props {
    readonly who: string;
    readonly at: number;

    readonly model: string | null;

    readonly ttft: Took | null;

    readonly tps: number | null;

    readonly rhythm: readonly number[] | null;
  }

  const { who, at, model, ttft, tps, rhythm }: Props = $props();

  const { lang } = ui();
  const first = $derived(
    ttft === null ? undefined : `${say($lang, "talk_ttft")} ${tookWords(ttft, $lang)}`,
  );
  const rate = $derived(
    tps === null ? undefined : fill(say($lang, "talk_tps"), { n: String(Math.round(tps)) }),
  );
</script>

{#snippet arrived()}
  {#if rhythm !== null}
    <Sparkline {rhythm} />
  {/if}
{/snippet}

<Look
  {who}
  {model}
  time={{ text: hhmmss(at), iso: isoInstant(at) }}
  ttft={first}
  tps={rate}
  rhythm={rhythm === null ? undefined : arrived}
/>
